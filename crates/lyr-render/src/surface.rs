//! Presenting a draw list on one window: crcbl-app's presenter and
//! crcbl-frame's UI pass, as `crates/crcbl-app/examples/ui_window.rs` does.

use std::sync::{Mutex, PoisonError};

use crcbl_app::adapter::PowerPreference;
use crcbl_app::backend::GpuBackend;
use crcbl_app::present::{FrameOutcome, PresentError, Presenter, PresenterDesc, SharedGpu};
use crcbl_frame::{
    GraphError, ImportedImage, InitialClaim, RenderGraph, TransientPool, UiRenderer,
};
use crcbl_hal::{CommandEncoderDesc, HalError, ResourceState};
use crcbl_shell::{Shell, ShellError, ShellEvent, WindowId};
use crcbl_ui::DrawList;
use crcbl_ui::image::ImageAtlas;
use crcbl_ui::text::FontAtlas;
use lyr_core::draw::DrawList as Bar;

use crate::pictures::Pictures;
use crate::text::Text;
use crate::translate::translate;

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
    #[error(transparent)]
    Shell(#[from] ShellError),
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

/// The device every surface after the first opens on, with the backend the
/// first was asked for. With a device each, closing one window's tore down
/// state another's still used on NVIDIA's Wayland driver, and lyr died with
/// a segfault when a monitor went (crcbl's backlog, "Two presenters crash
/// NVIDIA's Wayland WSI on teardown"). Held for the life of the process, as
/// the bar is.
static SHARED: Mutex<Option<(SharedGpu, Option<GpuBackend>)>> = Mutex::new(None);

/// One window's device, swapchain and UI pass.
pub struct Surface {
    presenter: Presenter,
    ui: UiRenderer,
    pool: TransientPool,
    /// The built-in bitmap font the UI pass wants; lyr's own text is glyph
    /// runs in its parsed font, so nothing draws from it.
    atlas: FontAtlas,
    /// The pictures this window's lists draw, on its UI pass's image atlas.
    pictures: Pictures,
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
        Self::open_on(shell, window, size, scale, preferred_backend())
    }

    /// [`Surface::open`], asking for `preferred` first.
    fn open_on(
        shell: &dyn Shell,
        window: WindowId,
        size: (u32, u32),
        scale: f32,
        preferred: Option<GpuBackend>,
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
        let mut shared = SHARED.lock().unwrap_or_else(PoisonError::into_inner);
        let on_shared = shared.as_ref().and_then(|(gpu, asked)| {
            // One the shared device cannot present to, a display wired to
            // another GPU, gets a device of its own below.
            gpu.open(shell, window, size, &desc(*asked))
                .ok()
                .map(|presenter| (presenter, *asked))
        });
        let (presenter, asked) = match on_shared {
            Some(opened) => opened,
            None => {
                let (presenter, asked) =
                    match Presenter::open(shell, window, size, &desc(preferred)) {
                        Ok(presenter) => (presenter, preferred),
                        // The preferred backend refused: let crcbl walk its
                        // own order.
                        Err(_) if preferred.is_some() => {
                            (Presenter::open(shell, window, size, &desc(None))?, None)
                        }
                        Err(error) => return Err(error.into()),
                    };
                if shared.is_none() {
                    *shared = Some((presenter.shared(), asked));
                }
                (presenter, asked)
            }
        };
        drop(shared);
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
            pictures: Pictures::default(),
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

    /// Window pixels per logical pixel, which [`crate::translate::translate`]
    /// records a list at.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
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

    /// Draws `bar`, lyr's list, as the window's next frame, its text in `text`
    /// and its pictures on this window's atlas: [`Surface::draw`] after
    /// [`translate`].
    ///
    /// # Errors
    ///
    /// As [`Surface::draw`].
    pub fn draw_bar(
        &mut self,
        shell: &mut dyn Shell,
        bar: &Bar,
        text: &Text,
    ) -> Result<bool, Error> {
        let (ui, pictures) = (&mut self.ui, &mut self.pictures);
        let list = translate(bar, text, self.scale, &mut |picture, width, height| {
            pictures.image(ui.images_mut(), picture, width, height)
        });
        self.pictures.sweep(self.ui.images_mut());
        self.draw(shell, &list)
    }

    /// Draws `list`, in logical pixels, as the window's next frame, asking
    /// `shell` to say when the window wants the one after. Returns whether
    /// it was presented: `false` when the swapchain was rebuilt instead, or
    /// the last frame's answer has not come ([`Surface::frame_due`]), and
    /// the caller has to draw again.
    ///
    /// Asking for each frame is what keeps a present to an output the
    /// compositor has turned off from blocking the bar: nothing is presented
    /// there until the compositor answers (crcbl-app's `present` module).
    ///
    /// # Errors
    ///
    /// When acquiring, recording or presenting fails, or the window has gone.
    pub fn draw(&mut self, shell: &mut dyn Shell, list: &DrawList) -> Result<bool, Error> {
        let Some(frame) = self.presenter.acquire()? else {
            return Ok(false);
        };
        let device = self.presenter.device();
        // The list is in window pixels already (`translate`'s scale), text
        // sizes included, so nothing is scaled again here.
        self.ui.begin_frame(device, list, &self.atlas, 1.0)?;
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
        // Before the present: on Wayland the request rides its commit.
        self.presenter.request_frame(shell)?;
        let outcome = self.presenter.submit_and_present(&frame, commands)?;
        Ok(outcome == FrameOutcome::Presented)
    }

    /// Reads one shell event for the answer to the last frame's request.
    /// Every pumped event goes through here, or the surface waits for an
    /// answer it never sees.
    pub fn observe(&mut self, event: &ShellEvent) {
        self.presenter.observe(event);
    }

    /// Whether [`Surface::draw`] would present now: `false` from a present
    /// until its answer is observed, while the bar has no reason to wake.
    #[must_use]
    pub const fn frame_due(&self) -> bool {
        self.presenter.frame_due()
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crcbl_app::backend::GpuBackend;
    use crcbl_app::window::{open_window, wait_for_configure};
    use crcbl_shell::{HeadlessShell, Shell, WindowDesc};
    use crcbl_ui::DrawList;

    use super::Surface;

    #[test]
    fn each_present_waits_for_the_shells_answer() {
        let mut shell = HeadlessShell::new();
        let window = open_window(&mut shell, Duration::ZERO, &WindowDesc::default())
            .expect("a headless window");
        let size = wait_for_configure(&mut shell, window, &mut |_| {}).expect("configured");
        let mut surface = Surface::open_on(&shell, window, size, 1.0, Some(GpuBackend::Null))
            .expect("a surface on the null backend");
        let list = DrawList::default();

        assert!(surface.frame_due());
        assert!(surface.draw(&mut shell, &list).expect("drawn"), "presented");
        assert!(!surface.frame_due(), "the present asked for the next frame");
        assert!(
            !surface.draw(&mut shell, &list).expect("skipped"),
            "nothing is presented before the answer"
        );

        shell.pump(&mut |event| surface.observe(&event));
        assert!(surface.frame_due(), "the answer came at the pump");
        assert!(surface.draw(&mut shell, &list).expect("drawn"));

        surface.destroy().expect("released");
        shell.destroy_window(window).expect("closed");
    }
}
