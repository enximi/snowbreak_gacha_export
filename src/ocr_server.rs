use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use base64::Engine;
use image::DynamicImage;
use paddleocr::{ImageData, Ppocr};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::{select, spawn};

fn image_to_base64(img: &DynamicImage) -> anyhow::Result<String> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| anyhow!("Failed to write image to buffer: {:?}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}

pub fn run_server() -> (JoinHandle<()>, JoinHandle<()>, OcrClient) {
    let (id_img_result_channel_tx, id_img_result_channel_rx) = mpsc::unbounded_channel();
    let (id_img_tx, id_img_rx) = mpsc::unbounded_channel();
    let (result_tx, result_rx) = mpsc::unbounded_channel();
    let mut ocr_server1 = OcrServer1 {
        result_tx,
        id_img_rx,
        ocr_processors: Arc::new(Mutex::new(vec![])),
    };
    let mut ocr_server2 = OcrServer2 {
        id_img_result_channel_rx,
        id_img_tx,
        result_rx,
        id_result_txs: HashMap::new(),
    };
    let ocr_client = OcrClient {
        id_img_result_channel_tx,
        last_id: 0,
    };
    let server1_handle = spawn(async move {
        ocr_server1.run().await;
    });
    let server2_handle = spawn(async move {
        ocr_server2.run().await;
    });
    (server1_handle, server2_handle, ocr_client)
}

/// 接受图片，返回结果
struct OcrServer1 {
    result_tx: mpsc::UnboundedSender<(u64, anyhow::Result<String>)>,
    id_img_rx: mpsc::UnboundedReceiver<(u64, DynamicImage)>,
    ocr_processors: Arc<Mutex<Vec<Ppocr>>>,
}

impl OcrServer1 {
    async fn run(&mut self) {
        let result_tx_clone1 = self.result_tx.clone();
        let ocr_processors_clone1 = self.ocr_processors.clone();
        while let Some((id, img)) = self.id_img_rx.recv().await {
            let result_tx_clone2 = result_tx_clone1.clone();
            let ocr_processors_clone2 = ocr_processors_clone1.clone();
            spawn(async move {
                let base64_img = match image_to_base64(&img) {
                    Ok(base64_img) => base64_img,
                    Err(e) => {
                        result_tx_clone2.send((id, Err(e))).unwrap();
                        return;
                    }
                };
                let mut ocr_processor = if ocr_processors_clone2.lock().unwrap().is_empty() {
                    match Ppocr::new(
                        PathBuf::from("./PaddleOCR-json_v.1.3.1/PaddleOCR-json.exe"),
                        Default::default(),
                    ) {
                        Ok(ocr_processor) => ocr_processor,
                        Err(e) => {
                            result_tx_clone2
                                .send((id, Err(anyhow!("Failed to create ocr_processor: {:?}", e))))
                                .unwrap();
                            return;
                        }
                    }
                } else {
                    match ocr_processors_clone2.lock().unwrap().pop() {
                        Some(ocr_processor) => ocr_processor,
                        None => {
                            result_tx_clone2
                                .send((id, Err(anyhow!("Failed to pop ocr_processor"))))
                                .unwrap();
                            return;
                        }
                    }
                };
                let result_json = match ocr_processor.ocr(ImageData::ImageBase64Dict {
                    image_base64: base64_img,
                }) {
                    Ok(result_json) => result_json,
                    Err(e) => {
                        result_tx_clone2
                            .send((id, Err(anyhow!("Failed to process image: {:?}", e))))
                            .unwrap();
                        return;
                    }
                };
                ocr_processors_clone2.lock().unwrap().push(ocr_processor);
                let json: serde_json::Value = match serde_json::from_str(&result_json) {
                    Ok(json) => json,
                    Err(e) => {
                        result_tx_clone2
                            .send((id, Err(anyhow!("Failed to parse json: {:?}", e))))
                            .unwrap();
                        return;
                    }
                };
                let code = match json["code"].as_i64() {
                    Some(code) => code,
                    None => {
                        result_tx_clone2
                            .send((id, Err(anyhow!("Failed to get code from json: {:?}", json))))
                            .unwrap();
                        return;
                    }
                };
                if code != 100 {
                    result_tx_clone2
                        .send((id, Err(anyhow!("Failed to process image: {:?}", json))))
                        .unwrap();
                    return;
                };
                let data = match json["data"].as_array() {
                    Some(data) => data,
                    None => {
                        result_tx_clone2
                            .send((id, Err(anyhow!("Failed to get data from json: {:?}", json))))
                            .unwrap();
                        return;
                    }
                };
                let item1 = match data.first() {
                    Some(item1) => item1,
                    None => {
                        result_tx_clone2
                            .send((
                                id,
                                Err(anyhow!("Failed to get item1 from data: {:?}", data)),
                            ))
                            .unwrap();
                        return;
                    }
                };
                let item1_text = match item1["text"].as_str() {
                    Some(item1_text) => item1_text,
                    None => {
                        result_tx_clone2
                            .send((
                                id,
                                Err(anyhow!("Failed to get text from item1: {:?}", item1)),
                            ))
                            .unwrap();
                        return;
                    }
                };
                result_tx_clone2
                    .send((id, Ok(item1_text.to_string())))
                    .unwrap();
            });
        }
    }
}

struct OcrServer2 {
    id_img_result_channel_rx:
        mpsc::UnboundedReceiver<(u64, DynamicImage, oneshot::Sender<anyhow::Result<String>>)>,
    id_img_tx: mpsc::UnboundedSender<(u64, DynamicImage)>,
    result_rx: mpsc::UnboundedReceiver<(u64, anyhow::Result<String>)>,
    id_result_txs: HashMap<u64, oneshot::Sender<anyhow::Result<String>>>,
}

impl OcrServer2 {
    async fn run(&mut self) {
        loop {
            select! {
                Some((id, img, result_tx)) = self.id_img_result_channel_rx.recv() => {
                    self.id_result_txs.insert(id, result_tx);
                    self.id_img_tx.send((id, img)).unwrap();
                },
                Some((id, result)) = self.result_rx.recv() => {
                    let result_tx = self.id_result_txs.remove(&id).unwrap();
                    result_tx.send(result).unwrap();
                },
            }
        }
    }
}

/// 编号并发送图片，留下接收结果的通道
pub struct OcrClient {
    id_img_result_channel_tx:
        mpsc::UnboundedSender<(u64, DynamicImage, oneshot::Sender<anyhow::Result<String>>)>,
    last_id: u64,
}

impl OcrClient {
    pub fn send(&mut self, img: DynamicImage) -> oneshot::Receiver<anyhow::Result<String>> {
        let (tx, rx) = oneshot::channel();
        self.id_img_result_channel_tx
            .send((self.last_id, img, tx))
            .unwrap();
        self.last_id += 1;
        rx
    }
}
