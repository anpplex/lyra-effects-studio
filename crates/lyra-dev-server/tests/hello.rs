use lyra_dev_server::{DevServer, DevServerEndpoint};
use lyra_device::HostPolicy;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct Response {
    status: u16,
    json: Value,
}

#[tokio::test]
async fn hello_rejects_missing_and_wrong_bearer_tokens() {
    let (server, endpoint) = start_test_server().await;

    let missing = send_hello(&endpoint, None, hello_fixture()).await;
    assert_response(&missing, 401, "device.bridge.unauthorized");
    let wrong = send_hello(&endpoint, Some("Bearer wrong"), hello_fixture()).await;
    assert_response(&wrong, 401, "device.bridge.unauthorized");

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn hello_preserves_the_portable_incompatible_protocol_code() {
    let (server, endpoint) = start_test_server().await;
    let authorization = endpoint.authorization_value();

    let response = send_hello(&endpoint, Some(&authorization), incompatible_fixture()).await;
    assert_response(&response, 422, "device.protocol.incompatible");

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn hello_requires_an_application_json_content_type() {
    let (server, endpoint) = start_test_server().await;
    let authorization = endpoint.authorization_value();

    let response = send_request(
        &endpoint,
        Some(&authorization),
        "text/plain",
        hello_fixture(),
    )
    .await;
    assert_response(&response, 400, "device.bridge.invalidRequest");

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn hello_creates_a_snapshot_with_the_negotiated_intersection() {
    let (server, endpoint) = start_test_server().await;

    let accepted = send_authorized_hello(&endpoint, hello_fixture()).await;
    assert_eq!(accepted.status, 200);
    assert_eq!(
        accepted.json["capabilities"],
        serde_json::json!(["activate", "stageRevision"])
    );
    assert_eq!(accepted.json["sessionId"].as_str().unwrap().len(), 32);
    let snapshot = server.session_snapshot().await.unwrap();
    assert_eq!(snapshot.device_profile_id, "com.avatr.cluster.4032x284");
    assert_eq!(snapshot.capabilities, ["activate", "stageRevision"]);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn same_profile_reconnect_is_idempotent_but_another_profile_is_rejected() {
    let (server, endpoint) = start_test_server().await;

    let first = send_authorized_hello(&endpoint, hello_fixture()).await;
    let repeat = send_authorized_hello(&endpoint, hello_fixture()).await;
    assert_eq!(first.status, 200);
    assert_eq!(first.json["sessionId"], repeat.json["sessionId"]);

    let another_profile = hello_with_profile("other.profile");
    let conflict = send_authorized_hello(&endpoint, &another_profile).await;
    assert_response(&conflict, 409, "device.bridge.sessionActive");
    assert_eq!(
        server.session_snapshot().await.unwrap().device_profile_id,
        "com.avatr.cluster.4032x284"
    );

    server.shutdown().await.unwrap();
}

async fn start_test_server() -> (DevServer, DevServerEndpoint) {
    let policy = HostPolicy::new(
        "1.2.0",
        ["activate", "hostOnly", "stageRevision"],
        ["stageRevision"],
    )
    .unwrap();
    DevServer::start(policy).await.unwrap()
}

fn hello_fixture() -> &'static [u8] {
    include_bytes!("../../../Fixtures/Device/hello-valid.json")
}

fn incompatible_fixture() -> &'static [u8] {
    include_bytes!("../../../Fixtures/Device/hello-incompatible.json")
}

fn hello_with_profile(device_profile_id: &str) -> Vec<u8> {
    let mut hello: Value = serde_json::from_slice(hello_fixture()).unwrap();
    hello["deviceProfileId"] = Value::String(device_profile_id.into());
    serde_json::to_vec(&hello).unwrap()
}

async fn send_authorized_hello(endpoint: &DevServerEndpoint, body: &[u8]) -> Response {
    let authorization = endpoint.authorization_value();
    send_hello(endpoint, Some(&authorization), body).await
}

async fn send_hello(
    endpoint: &DevServerEndpoint,
    authorization: Option<&str>,
    body: &[u8],
) -> Response {
    send_request(endpoint, authorization, "application/json", body).await
}

async fn send_request(
    endpoint: &DevServerEndpoint,
    authorization: Option<&str>,
    content_type: &str,
    body: &[u8],
) -> Response {
    let mut stream = TcpStream::connect(endpoint.address()).await.unwrap();
    let mut request = format!(
        "POST /v1/hello HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n",
        endpoint.address(),
        body.len()
    );
    if let Some(value) = authorization {
        request.push_str("Authorization: ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");

    stream.write_all(request.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
    stream.flush().await.unwrap();

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    decode_response(&raw)
}

fn decode_response(raw: &[u8]) -> Response {
    let separator = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or_else(|| {
            panic!(
                "response lacks an HTTP header: {}",
                String::from_utf8_lossy(raw)
            )
        });
    let headers = std::str::from_utf8(&raw[..separator]).unwrap();
    let status = headers
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse::<u16>()
        .unwrap();
    let json = serde_json::from_slice(&raw[separator + 4..]).unwrap();
    Response { status, json }
}

fn assert_response(response: &Response, status: u16, code: &str) {
    assert_eq!(response.status, status);
    assert_eq!(response.json["code"], code);
}
