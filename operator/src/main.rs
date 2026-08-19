use ::serde::{Serialize,Deserialize};
use std::{dbg, env, sync::{Arc,atomic::{AtomicBool,Ordering}}, time::Duration};
use tokio_tungstenite::{
    WebSocketStream, connect_async, tungstenite::{Message, WebSocket, client::IntoClientRequest}
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use futures_util::{Sink,TryStreamExt, sink::SinkExt, stream::{SplitSink, SplitStream, StreamExt}};
use http::{HeaderValue, header::{self, AUTHORIZATION}, request};
use rand::Rng;
// use std::collections::Vec;

//TODO: use tracing in place of eprintln! debug/error messages in async sections
//TODO: add a heartbeat to prevent disconnect on server inactivity
//TODO: graceful shutdown on ctrl+C
#[tokio::main]
async fn main()
{
    dotenvy::dotenv().ok();
    let host=std::env::var("OPERATOR_URL").unwrap_or_else(|_|"localhost".into());
    let port=std::env::var("OPERATOR_PORT").unwrap_or_else(|_|"3000".into());
    let server_url:String=format!("ws://{}:{}/ws",host.trim(),port.trim());
    let Ok(token)=env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Please provide an authorization token!"); 
        std::process::exit(1);
    }).parse::<HeaderValue>()
    else{
        eprintln!("Could not convert token to header value.");
        std::process::exit(1);
    };  

    //build and verify upgrade request
    let Ok(mut request)=server_url.clone().into_client_request() 
    else {
        eprintln!("Invalid request.");
        std::process::exit(1);
    };
    
    let reconnection_flag=Arc::new(AtomicBool::new(false));
    let (stdin_tx,mut stdin_rx)=mpsc::channel::<Message>(128);
    tokio::spawn(read_stdin(stdin_tx,reconnection_flag.clone()));
    request.headers_mut().insert(AUTHORIZATION, token);
    let mut delay=Duration::from_millis(1500);

    loop{
        //clear stdin so buffered messages don't get sent to server at a high rate
        while stdin_rx.try_recv().is_ok() {} 
        reconnection_flag.store(false,Ordering::Relaxed);
        match connect_async(request.clone()).await{
            Ok((server,response))=>
            {
                delay=Duration::from_millis(1500);
                eprintln!("Server responded with {:?}",response);
                let (mut tx, mut rx)=server.split();
                // let stdin_rx_ref=&mut stdin_rx;
                // let mut stdin_to_ws=tokio::spawn(send_ws(tx, &mut stdin_rx));
                let mut ws_to_stdout=tokio::spawn(async move{
                    while let Some(message)=rx.next().await{
                        match message{
                            Ok(msg)=>{
                                let mut stdout=tokio::io::stdout();
                                if let Err(err)=stdout.write_all(&msg.into_data()).await{
                                    eprintln!("Standard output failure:{}",err);
                                };
                                if let Err(err)=stdout.flush().await{
                                    eprintln!("Could not flush stdout: {}",err);
                                };
                            } 
                            Err(_)=>{
                                eprintln!("Message format error.");
                                break;
                            }
                        }
                    };
                });
                tokio::select!(
                    end_of_stream=send_ws(&mut tx, &mut stdin_rx)=>{
                        ws_to_stdout.abort();
                        if end_of_stream{
                            eprintln!("Standard input closed, exiting.");
                            return;
                        }
                    }
                    _=(&mut ws_to_stdout)=>{}
                );
            }
            Err(err)=>{
                eprintln!("Encountered error: {:?}",err);
                // break;
            }
        };
        //Reconnection with jitter
        let jitter=rand::rng().random_range(0..500);
        let wait=delay+Duration::from_millis(jitter);
        println!("Reconnecting... ");
        reconnection_flag.store(true, Ordering::Relaxed);
        tokio::time::sleep(wait).await;
        delay=(delay*2).min(Duration::from_secs(30));
    };
}

//Reads lines from standard inputs and passes them to buffer channel
//Intentionally drops inputs received during reconnections instead of passing to avoid stale inputs
async fn read_stdin(
    tx:tokio::sync::mpsc::Sender<Message>,
    reconnection_flag:Arc<AtomicBool>
){
    let mut reader=tokio::io::BufReader::new(tokio::io::stdin());
    let mut buffer=String::new();
    eprintln!("Start"); //DEBUG
    loop{
        buffer.clear();
        if reader.read_line(&mut buffer).await.unwrap_or_else(|_|0)!=0{
            if reconnection_flag.load(Ordering::Relaxed){
                println!("Reconnecting to server...message not sent!");
                continue;
            }
            let trim=buffer.trim_end().len();
            buffer.truncate(trim);
            eprintln!("output: {}",&buffer); //DEBUG
            if tx.send(Message::Text(buffer.clone().into())).await.is_err(){
                eprintln!("Channel dropped, resend message"); 
                break;
            }
        }else{
            eprintln!("EOF detected"); 
            break;
        };
    }
}

async fn send_ws<S>(
    sink: &mut S,
    stdin_rx:&mut tokio::sync::mpsc::Receiver<Message>,
)->bool
where
    S: Sink<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    while let Some(msg)=stdin_rx.recv().await{
        if sink.send(msg).await.is_err(){
            return false;
        }
    }
    true
}

