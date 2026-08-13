use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0; 1024];
                    if socket.read(&mut buf).await.is_ok() {
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                            <html>\
                            <head><title>CognyxOS Test Page</title></head>\
                            <body>\
                                <h1>CognyxOS Test Page</h1>\
                                <p>Hello from CognyxOS test server</p>\
                                <input type=\"text\" id=\"test-input\" />\
                                <button id=\"test-button\" onclick=\"document.getElementById('result').textContent = 'Button clicked! Input: ' + document.getElementById('test-input').value\">Click Me</button>\
                                <div id=\"result\"></div>\
                            </body>\
                            </html>";
                        let _ = socket.write_all(response.as_bytes()).await;
                    }
                });
            }
        }
    });

    (base_url, handle)
}
