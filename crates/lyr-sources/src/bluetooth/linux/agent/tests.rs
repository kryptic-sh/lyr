use std::os::unix::net::UnixStream;

use tokio::sync::mpsc;
use zbus::connection::Builder;
use zbus::{Connection, Guid, ObjectPath};

use super::{Agent, PATH, Prompt, Question};

const DEVICE: &str = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF";

/// How BlueZ calls the agent.
#[zbus::proxy(interface = "org.bluez.Agent1", default_path = "/sh/kryptic/lyr/agent")]
trait BlueZSide {
    fn request_confirmation(&self, device: &ObjectPath<'_>, passkey: u32) -> zbus::Result<()>;
    fn request_authorization(&self, device: &ObjectPath<'_>) -> zbus::Result<()>;
    fn request_pin_code(&self, device: &ObjectPath<'_>) -> zbus::Result<String>;
}

async fn agent() -> (
    BlueZSideProxy<'static>,
    mpsc::UnboundedReceiver<Prompt>,
    Connection,
) {
    let (prompts, asked) = mpsc::unbounded_channel();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(PATH, Agent { prompts })
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    let bluez = BlueZSideProxy::builder(&client)
        .destination("org.bluez")
        .build()
        .await
        .expect("proxy");
    (bluez, asked, server)
}

/// The name of the D-Bus error `error` is, if it is one.
fn error_name(error: &zbus::Error) -> Option<String> {
    match error {
        zbus::Error::MethodError(name, _, _) => Some(name.to_string()),
        _ => None,
    }
}

#[tokio::test]
async fn a_confirmed_passkey_lets_pairing_go_on() {
    let (bluez, mut asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let answer = async {
        let prompt = asked.recv().await.expect("a prompt");
        assert_eq!(
            (prompt.device.as_str(), prompt.question),
            (DEVICE, Question::Confirm(123_456))
        );
        prompt.answer.send(true).expect("answered");
    };
    let (asked_bluez, ()) = tokio::join!(bluez.request_confirmation(&device, 123_456), answer);
    asked_bluez.expect("confirmed");
}

#[tokio::test]
async fn a_declined_or_unanswered_question_is_rejected_or_canceled() {
    let (bluez, mut asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let decline = async {
        let prompt = asked.recv().await.expect("a prompt");
        assert_eq!(prompt.question, Question::Authorize);
        prompt.answer.send(false).expect("answered");
    };
    let (declined, ()) = tokio::join!(bluez.request_authorization(&device), decline);
    assert_eq!(
        error_name(&declined.expect_err("declined")).as_deref(),
        Some("org.bluez.Error.Rejected")
    );
    let ignore = async {
        drop(asked.recv().await.expect("a prompt"));
    };
    let (ignored, ()) = tokio::join!(bluez.request_authorization(&device), ignore);
    assert_eq!(
        error_name(&ignored.expect_err("canceled")).as_deref(),
        Some("org.bluez.Error.Canceled")
    );
}

#[tokio::test]
async fn a_pin_is_rejected_since_lyr_cannot_take_one() {
    let (bluez, _asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let error = bluez.request_pin_code(&device).await.expect_err("rejected");
    assert_eq!(
        error_name(&error).as_deref(),
        Some("org.bluez.Error.Rejected")
    );
}
