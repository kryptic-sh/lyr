//! lyr's Bluetooth pairing agent (`org.bluez.Agent1`, BlueZ's
//! `doc/org.bluez.Agent.rst`), so pairing questions are answered in the
//! Bluetooth menu (`docs/plan/10-popups-and-menus.md#bluetooth-menu`).
//!
//! It registers as `DisplayYesNo`: it confirms a passkey, authorizes a
//! device or a service, and takes a PIN typed into the menu. BlueZ asks for
//! a PIN of a device that pairs the legacy way whatever the agent's
//! capability (`pin_code_request_callback` in its `src/adapter.c`), while it
//! asks for a typed passkey only of an agent with a keyboard, which lyr does
//! not claim; that request is rejected. Each question goes to the bar as a
//! [`Prompt`], and the D-Bus call waits for the answer, or for BlueZ to give
//! up on it ([`Asked::Cancel`]).

use std::collections::HashMap;

pub use lyr_core::bluetooth_menu::{Question, Reply};
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
    pub answer: oneshot::Sender<Reply>,
}

/// What the agent tells the bar.
#[derive(Debug)]
pub enum Asked {
    Prompt(Prompt),
    /// BlueZ gave up on the question it asked last. It asks one at a time
    /// and cancels before asking again (`agent_cancel` in its
    /// `src/agent.c`), so this is always that one.
    Cancel,
}

/// The questions waiting for an answer, by the id the bar knows each by.
#[derive(Debug, Default)]
pub struct Questions {
    last: u64,
    pending: HashMap<u64, oneshot::Sender<Reply>>,
}

impl Questions {
    /// Keeps a question's `answer` until it is answered, returning the id
    /// to answer it by.
    pub fn ask(&mut self, answer: oneshot::Sender<Reply>) -> u64 {
        self.last += 1;
        self.pending.insert(self.last, answer);
        self.last
    }

    /// Drops every question waiting: BlueZ gave up on them.
    pub fn cancel(&mut self) {
        self.pending.clear();
    }

    /// Answers question `id` with `reply`: `false` when nothing waits on
    /// it any more.
    pub fn answer(&mut self, id: u64, reply: Reply) -> bool {
        self.pending
            .remove(&id)
            .is_some_and(|answer| answer.send(reply).is_ok())
    }
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
    pub prompts: mpsc::UnboundedSender<Asked>,
}

impl Agent {
    /// Asks `question` about `device` and waits for the answer, a No being
    /// BlueZ's `Rejected`.
    async fn ask(&self, device: OwnedObjectPath, question: Question) -> Result<Reply, AgentError> {
        let (answer, answered) = oneshot::channel();
        let prompt = Prompt {
            device: device.to_string(),
            question,
            answer,
        };
        if self.prompts.send(Asked::Prompt(prompt)).is_err() {
            return Err(AgentError::Canceled("lyr is exiting".to_owned()));
        }
        match answered.await {
            Ok(Reply::No) => Err(AgentError::Rejected("declined in lyr".to_owned())),
            Ok(reply) => Ok(reply),
            Err(_) => Err(AgentError::Canceled("not answered".to_owned())),
        }
    }

    /// Asks a yes or no `question` about `device`.
    async fn confirm(&self, device: OwnedObjectPath, question: Question) -> Result<(), AgentError> {
        match self.ask(device, question).await? {
            Reply::Yes => Ok(()),
            Reply::No | Reply::Pin(_) => Err(AgentError::Rejected("not a yes".to_owned())),
        }
    }
}

#[zbus::interface(name = "org.bluez.Agent1")]
impl Agent {
    fn release(&self) {}

    async fn request_pin_code(&self, device: OwnedObjectPath) -> Result<String, AgentError> {
        match self.ask(device, Question::Pin).await? {
            Reply::Pin(pin) => Ok(pin),
            Reply::Yes | Reply::No => Err(AgentError::Rejected("no PIN typed".to_owned())),
        }
    }

    fn display_pin_code(&self, _device: OwnedObjectPath, _pincode: String) {}

    fn request_passkey(&self, _device: OwnedObjectPath) -> Result<u32, AgentError> {
        Err(AgentError::Rejected(
            "lyr has no keyboard capability".to_owned(),
        ))
    }

    fn display_passkey(&self, _device: OwnedObjectPath, _passkey: u32, _entered: u16) {}

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> Result<(), AgentError> {
        self.confirm(device, Question::Confirm(passkey)).await
    }

    async fn request_authorization(&self, device: OwnedObjectPath) -> Result<(), AgentError> {
        self.confirm(device, Question::Authorize).await
    }

    async fn authorize_service(
        &self,
        device: OwnedObjectPath,
        _uuid: String,
    ) -> Result<(), AgentError> {
        self.confirm(device, Question::AuthorizeService).await
    }

    fn cancel(&self) {
        // Nobody to tell when the bar is exiting.
        let _exiting = self.prompts.send(Asked::Cancel);
    }
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
