use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::{Header, Response, Server, StatusCode};

#[test]
fn check_command_reports_success_redirect_and_errors() {
    let server = TestServer::start();

    let output = Command::new(env!("CARGO_BIN_EXE_sitepulse"))
        .args([
            "check",
            &server.url("/sitemap.xml"),
            "--concurrency",
            "2",
            "--timeout",
            "5",
            "--retries",
            "1",
        ])
        .output()
        .expect("failed to run sitepulse binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Discovered URLs: 4"), "{stdout}");
    assert!(stdout.contains("Summary:"), "{stdout}");
    assert!(stdout.contains("2xx: 2"), "{stdout}");
    assert!(stdout.contains("3xx: 0"), "{stdout}");
    assert!(stdout.contains("4xx: 1"), "{stdout}");
    assert!(stdout.contains("5xx: 1"), "{stdout}");
    assert!(stdout.contains("/redirect ->"), "{stdout}");
}

#[test]
fn fail_on_errors_exits_with_code_two() {
    let server = TestServer::start();

    let output = Command::new(env!("CARGO_BIN_EXE_sitepulse"))
        .args([
            "check",
            &server.url("/sitemap.xml"),
            "--max-urls",
            "1",
            "--fail-on-errors",
        ])
        .output()
        .expect("failed to run sitepulse binary");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn head_method_checks_urls() {
    let server = TestServer::start();

    let output = Command::new(env!("CARGO_BIN_EXE_sitepulse"))
        .args([
            "check",
            &server.url("/sitemap.xml"),
            "--method",
            "head",
            "--max-urls",
            "2",
        ])
        .output()
        .expect("failed to run sitepulse binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Method: HEAD"), "{stdout}");
    assert!(stdout.contains("HEAD"), "{stdout}");
}

struct TestServer {
    base_url: String,
    addr: SocketAddr,
}

impl TestServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind test server");
        let addr = listener
            .local_addr()
            .expect("failed to read test server addr");
        let server = Server::from_listener(listener, None).expect("failed to start test server");
        let base_url = format!("http://{}", addr);
        let requests = Arc::new(Mutex::new(0));
        let thread_requests = Arc::clone(&requests);
        let thread_base_url = base_url.clone();

        thread::spawn(move || {
            for request in server.incoming_requests() {
                *thread_requests.lock().expect("request count lock poisoned") += 1;
                let path = request.url().to_string();
                let response = match path.as_str() {
                    "/sitemap.xml" => xml_response(format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{}/missing</loc></url>
  <url><loc>{}/ok</loc></url>
  <url><loc>{}/redirect</loc></url>
  <url><loc>{}/server-error</loc></url>
</urlset>"#,
                        thread_base_url, thread_base_url, thread_base_url, thread_base_url
                    )),
                    "/ok" | "/final" => Response::from_string("ok"),
                    "/missing" => {
                        Response::from_string("missing").with_status_code(StatusCode(404))
                    }
                    "/server-error" => {
                        Response::from_string("error").with_status_code(StatusCode(500))
                    }
                    "/redirect" => Response::from_string("")
                        .with_status_code(StatusCode(301))
                        .with_header(
                            Header::from_bytes("Location", format!("{}/final", thread_base_url))
                                .expect("failed to build location header"),
                        ),
                    _ => Response::from_string("not found").with_status_code(StatusCode(404)),
                };
                let _ = request.respond(response);
            }
        });

        Self { base_url, addr }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = TcpStream::connect(self.addr);
    }
}

fn xml_response(body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body).with_header(
        Header::from_bytes("Content-Type", "application/xml").expect("failed to build header"),
    )
}
