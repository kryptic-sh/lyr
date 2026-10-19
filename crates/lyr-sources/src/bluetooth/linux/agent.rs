//! lyr's Bluetooth pairing agent (`org.bluez.Agent1`, BlueZ's
//! `doc/org.bluez.Agent.rst`), so pairing questions are answered in the
//! Bluetooth menu (`docs/plan/10-popups-and-menus.md#bluetooth-menu`).
//!
//! It registers as `DisplayYesNo`: lyr has no text input yet, so it can
//! confirm a passkey and authorize a device or a service, but cannot type a
//! PIN or a passkey; BlueZ's requests for those are rejected. Each question
//! goes to the bar as a [`Prompt`], and the D-Bus call waits for the answer.

pub use lyr_core::bluetooth_menu::Question;
use tokio::sync::{mpsc, oneshot};
use zbus::{Connection, ObjectPath, OwnedObjectPath};

/// Where lyr serves its agent.
pub const PATH: &str = "/sh/kryptic/lyr/agent";
/// What the agent can do: show a yes or no question.
pub const CAPABILITY: &str = "DisplayYesNo";

/// One question for the user, about the device at `device`; answer it on
/// `answer`. Dropping it unanswered cancels the request.
#[derive(Debug)]
pub struct Prompt {
    pub device: String,
    pub question: Question,
    pub answer: oneshot::Sender<bool>,
}

/// The agent's errors, as BlueZ names them.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bluez.Error")]
pub enum AgentError {
    #[zbus(error)]
    ZBus(zbus::Error),
    Rejected(String),
    Canceled(String),
}

/// The agent: where its questions go.
#[derive(Debug)]
pub struct Agent {
    pub prompts: mpsc::UnboundedSender<Prompt>,
}

impl Agent {
    /// Asks `question` about `device` and waits for the answer.
    async fn ask(&self, device: OwnedObjectPath, question: Question) -> Result<(), AgentError> {
        let (answer, answered) = oneshot::channel();
        let prompt = Prompt {
            device: device.to_string(),
            question,
            answer,
        };
        if self.prompts.send(prompt).is_err() {
            return Err(AgentError::Canceled("lyr is exiting".to_owned()));
        }
        match answered.await {
            Ok(true) => Ok(()),
            Ok(false) => Err(AgentError::Rejected("declined in lyr".to_owned())),
            Err(_) => Err(AgentError::Canceled("not answered".to_owned())),
        }
    }
}

#[zbus::interface(name = "org.bluez.Agent1")]
impl Agent {
    fn release(&self) {}

    fn request_pin_code(&self, _device: OwnedObjectPath) -> Result<String, AgentError> {
        Err(AgentError::Rejected("lyr cannot take a PIN yet".to_owned()))
    }

    fn display_pin_code(&self, _device: OwnedObjectPath, _pincode: String) {}

    fn request_passkey(&self, _device: OwnedObjectPath) -> Result<u32, AgentError> {
        Err(AgentError::Rejected(
            "lyr cannot take a passkey yet".to_owned(),
        ))
    }

    fn display_passkey(&self, _device: OwnedObjectPath, _passkey: u32, _entered: u16) {}

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> Result<(), AgentError> {
        self.ask(device, Question::Confirm(passkey)).await
    }

    async fn request_authorization(&self, device: OwnedObjectPath) -> Result<(), AgentError> {
        self.ask(device, Question::Authorize).await
    }

    async fn authorize_service(
        &self,
        device: OwnedObjectPath,
        _uuid: String,
    ) -> Result<(), AgentError> {
        self.ask(device, Question::AuthorizeService).await
    }

    fn cancel(&self) {}
}

#[zbus::proxy(
    interface = "org.bluez.AgentManager1",
    default_service = "org.bluez",
    default_path = "/org/bluez"
)]
trait AgentManager {
    fn register_agent(&self, agent: &ObjectPath<'_>, capability: &str) -> zbus::Result<()>;
    fn request_default_agent(&self, agent: &ObjectPath<'_>) -> zbus::Result<()>;
}

/// Serves `agent` on `connection` and registers it with BlueZ as the
/// default agent.
///
/// # Errors
///
/// When it cannot be served or BlueZ refuses it.
pub async fn register(connection: &Connection, agent: Agent) -> zbus::Result<()> {
    connection.object_server().at(PATH, agent).await?;
    let manager = AgentManagerProxy::new(connection).await?;
    let path = ObjectPath::try_from(PATH)?;
    manager.register_agent(&path, CAPABILITY).await?;
    manager.request_default_agent(&path).await
}

#[cfg(test)]
mod tests;
