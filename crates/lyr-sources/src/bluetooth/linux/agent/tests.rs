use std::os::unix::net::UnixStream;
use std::time::Duration;

use tokio::sync::mpsc;
use zbus::connection::Builder;
use zbus::{Connection, Guid, ObjectPath};

use super::{Agent, Asked, PATH, Prompt, Question, Questions, Reply};

const DEVICE: &str = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF";

/// How BlueZ calls the agent.
#[zbus::proxy(interface = "org.bluez.Agent1", default_path = "/sh/kryptic/lyr/agent")]
trait BlueZSide {
    fn request_confirmation(&self, device: &ObjectPath<'_>, passkey: u32) -> zbus::Result<()>;
    fn request_authorization(&self, device: &ObjectPath<'_>) -> zbus::Result<()>;
    fn request_pin_code(&self, device: &ObjectPath<'_>) -> zbus::Result<String>;
    fn request_passkey(&self, device: &ObjectPath<'_>) -> zbus::Result<u32>;
    fn cancel(&self) -> zbus::Result<()>;
}

/// The next question the agent passes on.
async fn prompt(asked: &mut mpsc::UnboundedReceiver<Asked>) -> Prompt {
    match asked.recv().await.expect("a question") {
        Asked::Prompt(prompt) => prompt,
        Asked::Cancel => panic!("a cancel, not a question"),
    }
}

async fn agent() -> (
    BlueZSideProxy<'static>,
    mpsc::UnboundedReceiver<Asked>,
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
        let prompt = prompt(&mut asked).await;
        assert_eq!(
            (prompt.device.as_str(), prompt.question),
            (DEVICE, Question::Confirm(123_456))
        );
        prompt.answer.send(Reply::Yes).expect("answered");
    };
    let (asked_bluez, ()) = tokio::join!(bluez.request_confirmation(&device, 123_456), answer);
    asked_bluez.expect("confirmed");
}

#[tokio::test]
async fn a_declined_or_unanswered_question_is_rejected_or_canceled() {
    let (bluez, mut asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let decline = async {
        let prompt = prompt(&mut asked).await;
        assert_eq!(prompt.question, Question::Authorize);
        prompt.answer.send(Reply::No).expect("answered");
    };
    let (declined, ()) = tokio::join!(bluez.request_authorization(&device), decline);
    assert_eq!(
        error_name(&declined.expect_err("declined")).as_deref(),
        Some("org.bluez.Error.Rejected")
    );
    let ignore = async {
        drop(prompt(&mut asked).await);
    };
    let (ignored, ()) = tokio::join!(bluez.request_authorization(&device), ignore);
    assert_eq!(
        error_name(&ignored.expect_err("canceled")).as_deref(),
        Some("org.bluez.Error.Canceled")
    );
}

#[tokio::test]
async fn a_pin_typed_in_goes_to_bluez() {
    let (bluez, mut asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let answer = async {
        let prompt = prompt(&mut asked).await;
        assert_eq!(prompt.question, Question::Pin);
        prompt
            .answer
            .send(Reply::Pin("0042".to_owned()))
            .expect("answered");
    };
    let (pin, ()) = tokio::join!(bluez.request_pin_code(&device), answer);
    assert_eq!(pin.expect("a PIN"), "0042");
    let yes = async {
        let prompt = prompt(&mut asked).await;
        prompt.answer.send(Reply::Yes).expect("answered");
    };
    let (no_pin, ()) = tokio::join!(bluez.request_pin_code(&device), yes);
    assert_eq!(
        error_name(&no_pin.expect_err("rejected")).as_deref(),
        Some("org.bluez.Error.Rejected"),
        "a yes is no PIN"
    );
}

#[tokio::test]
async fn a_typed_passkey_is_rejected_since_lyr_claims_no_keyboard() {
    let (bluez, _asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let error = bluez.request_passkey(&device).await.expect_err("rejected");
    assert_eq!(
        error_name(&error).as_deref(),
        Some("org.bluez.Error.Rejected")
    );
}

#[tokio::test]
async fn bluez_giving_up_is_passed_on() {
    let (bluez, mut asked, _server) = agent().await;
    bluez.cancel().await.expect("cancelled");
    let passed = tokio::time::timeout(Duration::from_secs(5), asked.recv())
        .await
        .expect("passed on in time");
    assert!(matches!(passed, Some(Asked::Cancel)));
}

#[tokio::test]
async fn an_answer_reaches_bluez_by_its_question_s_id() {
    let (bluez, mut asked, _server) = agent().await;
    let device = ObjectPath::try_from(DEVICE).expect("a path");
    let mut questions = Questions::default();
    let answer = async {
        let first = questions.ask(prompt(&mut asked).await.answer);
        assert!(!questions.answer(first + 1, Reply::Yes), "no such question");
        assert!(questions.answer(first, Reply::Yes));
        assert!(!questions.answer(first, Reply::Yes), "answered once");
        let second = questions.ask(prompt(&mut asked).await.answer);
        assert_ne!(second, first);
        questions.cancel();
        assert!(!questions.answer(second, Reply::Yes), "BlueZ gave up on it");
    };
    let confirm = async {
        let first = bluez.request_confirmation(&device, 1).await;
        let second = bluez.request_authorization(&device).await;
        (first, second)
    };
    let ((first, second), ()) = tokio::join!(confirm, answer);
    first.expect("confirmed");
    assert_eq!(
        error_name(&second.expect_err("canceled")).as_deref(),
        Some("org.bluez.Error.Canceled")
    );
}
