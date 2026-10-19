//! Presenting a draw list on one window: crcbl-app's presenter and
//! crcbl-frame's UI pass, as `crates/crcbl-app/examples/ui_window.rs` does.

use crcbl_app::adapter::PowerPreference;
use crcbl_app::backend::GpuBackend;
use crcbl_app::present::{FrameOutcome, PresentError, Presenter, PresenterDesc};
use crcbl_frame::{
    GraphError, ImportedImage, InitialClaim, RenderGraph, TransientPool, UiRenderer,
};
use crcbl_hal::{CommandEncoderDesc, HalError, ResourceState};
use crcbl_shell::{Shell, WindowId};
use crcbl_ui::DrawList;
use crcbl_ui::image::ImageAtlas;
use crcbl_ui::text::FontAtlas;

/// What the cleared window is before the bar draws: fully transparent, so
/// whatever the bar leaves undrawn shows the desktop.
const CLEAR: [f32; 4] = [0.0; 4];

/// Why a frame or a surface failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Present(#[from] PresentError),
    #[error(transparent)]
    Hal(#[from] HalError),
    #[error(transparent)]
    Graph(#[from] GraphError),
}

/// The backend lyr asks for first: D3D12 on Windows, where only it gives a
/// window per-pixel transparency, and crcbl's own order elsewhere.
const fn preferred_backend() -> Option<GpuBackend> {
    if cfg!(windows) {
        Some(GpuBackend::Dx12)
    } else {
        None
    }
}

/// One window's device, swapchain and UI pass.
pub struct Surface {
    presenter: Presenter,
    ui: UiRenderer,
    pool: TransientPool,
    /// The built-in bitmap font the UI pass wants; lyr's own text is glyph
    /// runs in its parsed font, so nothing draws from it.
    atlas: FontAtlas,
    /// Device pixels per logical pixel.
    scale: f32,
    /// The backend asked for and accepted, or `None` when crcbl's own order
    /// chose (the preferred backend refused, or there is none).
    asked: Option<GpuBackend>,
}

impl Surface {
    /// Opens a surface on `window`, `size` device pixels at `scale`.
    ///
    /// # Errors
    ///
    /// When no backend can present to the window, or the UI pass cannot be
    /// created.
    pub fn open(
        shell: &dyn Shell,
        window: WindowId,
        size: (u32, u32),
        scale: f32,
    ) -> Result<Self, Error> {
        let adapter_pin = crcbl_app::adapter::pin();
        // The owner's rule for the GPU: `CRCBL_ADAPTER` when set, then the
        // adapter driving the bar's display when it is discrete, then low
        // power. The presenter takes it through `crcbl_app::adapter::choose`
        // with the display's adapter; low power is named so a change to
        // crcbl's default cannot wake a discrete GPU for a bar.
        let desc = |backend| PresenterDesc {
            label: "lyr",
            backend,
            adapter_pin: adapter_pin.as_deref(),
            power_preference: PowerPreference::LowPower,
            ..PresenterDesc::default()
        };
        let (presenter, asked) =
            match Presenter::open(shell, window, size, &desc(preferred_backend())) {
                Ok(presenter) => (presenter, preferred_backend()),
                // The preferred backend refused: let crcbl walk its own order.
                Err(_) if preferred_backend().is_some() => {
                    (Presenter::open(shell, window, size, &desc(None))?, None)
                }
                Err(error) => return Err(error.into()),
            };
        let ui = UiRenderer::with_ring(
            presenter.device(),
            presenter.queue(),
            presenter.format(),
            ImageAtlas::new(),
            presenter.frames_in_flight(),
        )?;
        Ok(Self {
            presenter,
            ui,
            pool: TransientPool::new(),
            atlas: FontAtlas::built_in(),
            scale,
            asked,
        })
    }

    /// Which GPU this surface draws on and how it was chosen, for the log:
    /// the adapter's name, and whether the preferred backend was accepted.
    #[must_use]
    pub fn describe(&self) -> String {
        let adapter = self
            .presenter
            .adapter()
            .map_or_else(|| "an unknown adapter".to_owned(), |info| info.name);
        match self.asked {
            Some(backend) => format!("{adapter} ({backend:?}, as asked)"),
            None if preferred_backend().is_some() => {
                format!("{adapter} (crcbl's order: the preferred backend refused)")
            }
            None => format!("{adapter} (crcbl's order)"),
        }
    }

    /// Follows the window to a new size or scale.
    ///
    /// # Errors
    ///
    /// When the swapchain cannot be rebuilt.
    pub fn resize(&mut self, size: (u32, u32), scale: f32) -> Result<(), Error> {
        self.scale = scale;
        self.presenter.resize(size)?;
        Ok(())
    }

    /// Draws `list`, in logical pixels, as the window's next frame. Returns
    /// whether it was presented: `false` when the swapchain was rebuilt
    /// instead, and the caller has to draw again.
    ///
    /// # Errors
    ///
    /// When acquiring, recording or presenting fails.
    pub fn draw(&mut self, list: &DrawList) -> Result<bool, Error> {
        let Some(frame) = self.presenter.acquire()? else {
            return Ok(false);
        };
        let device = self.presenter.device();
        self.ui.begin_frame(device, list, &self.atlas, self.scale)?;
        let mut graph = RenderGraph::new(self.presenter.queue());
        let target = graph.import_image(
            "swapchain",
            ImportedImage {
                image: frame.image,
                view: frame.view,
                format: self.presenter.format(),
                extent: frame.extent,
                initial: ResourceState::Undefined,
                claim: InitialClaim::Acquired,
                final_state: ResourceState::Present,
            },
        );
        graph
            .add_render_pass("clear")
            .clear_color(target, CLEAR)
            .execute(|_| {});
        self.ui.add_passes(&mut graph, target, frame.extent);
        let compiled = graph.compile(&self.pool)?;
        let mut encoder = device.create_command_encoder(&CommandEncoderDesc {
            label: Some("lyr frame"),
            queue: self.presenter.queue(),
        });
        compiled.execute(device, &mut self.pool, encoder.as_mut(), None)?;
        let commands = encoder.finish()?;
        let outcome = self.presenter.submit_and_present(&frame, commands)?;
        Ok(outcome == FrameOutcome::Presented)
    }

    /// Waits for the device and releases everything, in the order the example
    /// does.
    ///
    /// # Errors
    ///
    /// When the device cannot be drained or the presenter destroyed.
    pub fn destroy(mut self) -> Result<(), Error> {
        self.presenter.drain()?;
        self.ui.destroy(self.presenter.device());
        self.pool.destroy(self.presenter.device());
        self.presenter.destroy()?;
        Ok(())
    }
}
