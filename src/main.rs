use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use local_ip_address::local_ip;
use std::env;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body(
        r#"
        <!doctype html>
        <html>
        <head>
            <title>Upload File</title>
            <style>
                body {
                    font-family: Arial, sans-serif;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 100vh;
                    background-color: #f4f4f4;
                    margin: 0;
                    padding: 0;
                }

                #uploadForm {
                    text-align: center;
                    padding: 20px;
                    border: 2px dashed #ddd;
                    background-color: #fff;
                    position: relative;
                    width: 80%;
                    height: 80%;
                    margin: auto;
                }

                #uploadForm.dragover {
                    border: 2px dashed #3498db;
                }

                #uploadForm.dragover::after {
                    content: 'Drop here';
                    position: absolute;
                    top: 50%;
                    left: 50%;
                    transform: translate(-50%, -50%);
                    font-size: 20px;
                    color: #3498db;
                }

                #fileName {
                    font-size: 1.5em;
                    margin-bottom: 10px;
                }

                #fileInput {
                    display: none;
                }

                #fileLabel {
                    display: inline-block;
                    margin-bottom: 20px;
                    padding: 10px 20px;
                    border: 1px solid #ddd;
                    border-radius: 5px;
                    background-color: #f4f4f4;
                    cursor: pointer;
                }

                #submitButton {
                    display: inline-block;
                    padding: 10px 20px;
                    border: none;
                    border-radius: 5px;
                    background-color: #3498db;
                    color: #fff;
                    cursor: pointer;
                    font-size: 1.2em;
                }

                #progressBar {
                    position: absolute;
                    bottom: 0;
                    left: 0;
                    width: 0%;
                    height: 20px;
                    background: green;
                    transition: width .3s ease-in-out;
                }
            </style>
            <script>
                                window.onload = function() {
                    var uploadForm = document.getElementById('uploadForm');
                    var fileInput = document.getElementById('fileInput');
                    var fileName = document.getElementById('fileName');
                    var progressBar = document.getElementById('progressBar');

                    uploadForm.addEventListener('submit', function(event) {
                        event.preventDefault();
                        var files = fileInput.files;
                        Array.from(files).forEach(uploadFile);
                    });

                    fileInput.addEventListener('change', function() {
                        fileName.textContent = Array.from(fileInput.files).map(file => file.name).join(', ');
                    });

                    uploadForm.addEventListener('dragenter', function(event) {
                        uploadForm.classList.add('dragover');
                    }, false);

                    uploadForm.addEventListener('dragleave', function(event) {
                        uploadForm.classList.remove('dragover');
                    }, false);

                    window.addEventListener('dragover', function(event) {
                        event.preventDefault();
                    }, false);

                    window.addEventListener('drop', function(event) {
                        event.preventDefault();
                        var files = event.dataTransfer.files;
                        Array.from(files).forEach(uploadFile);
                        uploadForm.classList.remove('dragover');
                    }, false);
                }

                function uploadFile(file) {
                    var formData = new FormData();
                    formData.append('file', file);
                    fetch('/upload_file', { method: 'POST', body: formData }).then(response => {
                        const reader = response.body.getReader();
                        const contentLength = +response.headers.get('Content-Length');
                        let receivedLength = 0;

                        reader.read().then(function processChunk({ done, value }) {
                            if (done) {
                                progressBar.style.width = '100%';
                                setTimeout(function() {
                                    progressBar.style.width = '0%';
                                    uploadForm.reset();
                                    fileName.textContent = '';
                                }, 1000);
                                return;
                            }

                            receivedLength += value.length;
                            const percentComplete = (receivedLength / contentLength) * 100;
                            progressBar.style.width = percentComplete + '%';

                            return reader.read().then(processChunk);
                        });
                    });
                }
            </script>
        </head>
        <body>
            <form id="uploadForm" action="/upload_file" method="post" enctype="multipart/form-data">
                <div id="fileName"></div>
                <input id="fileInput" type="file" name="file" multiple/>
                <label id="fileLabel" for="fileInput">Select file</label>
                <input id="submitButton" type="submit" value="Upload" />
            </form>
            <div id="progressBar"></div>
        </body>
        </html>
        "#,
    )
}

#[post("/upload_file")]
async fn upload_file(mut payload: Multipart) -> impl Responder {
    let mut file_path = String::new();

    while let Some(item) = payload.next().await {
        match item {
            Ok(mut field) => {
                let content_disposition = field.content_disposition();
                let filename = match content_disposition.get_filename() {
                    Some(name) => name,
                    None => return HttpResponse::BadRequest().body("No filename provided"),
                };
                file_path = format!("./{}", &filename);

                if tokio::fs::metadata(&file_path).await.is_ok() {
                    // Split once on . once
                    let (name, ext) = filename.split_once('.').unwrap();

                    // Append _new to the filename before the extension
                    file_path = format!("./{}_new.{}", name, ext);
                }

                let mut file = Some(File::create(&file_path).await.unwrap());

                while let Some(chunk) = field.next().await {
                    let data = chunk.unwrap();
                    if let Some(file) = &mut file {
                        file.write_all(&data).await.unwrap();
                    }
                }

                println!("File uploaded to '{}'", file_path);
            }
            Err(e) => return HttpResponse::InternalServerError().body(format!("Server error: {}", e)),
        }
    }

    HttpResponse::Ok().body(format!("File uploaded to '{}'", file_path))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut port = 8080;

    for i in 1..args.len() {
        match args[i].as_str() {
            "-p" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().expect("Invalid port number");
                } else {
                    println!("The -p option requires a port number.");
                    return Ok(());
                }
            },
            "-h" => {
                println!("Usage: \n-p [port number]: Set the port number for the server\n-h: Display this help message");
                return Ok(());
            },
            _ => {}
        }
    }

    let server = HttpServer::new(|| {
        App::new()
            .service(index)
            .service(upload_file)
    })
    .bind(("0.0.0.0", port))?;

    println!("Server running at http://{:?}:{}", local_ip().unwrap(), port);

    server.run().await
}