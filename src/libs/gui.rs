use std::vec;
use std::{net::TcpStream};
use std::io::{prelude::*, BufReader, BufWriter};
use eframe::{egui};
use egui::{menu, style::Margin, Vec2, Color32};
use ini::Ini;
use serde::{Serialize, Deserialize};
use bincode;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use std::thread::sleep as sleep;
use std::time::Duration as Duration;
use std::sync::mpsc::{self};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

const CHAT_MAX_SIZE: usize = 10;

#[derive(Default, Debug)]
pub struct Client {
    id: String,
    inited: bool,
    connected: bool,
    connected_to: Channel,
    connection: Option<TcpStream>,
    chat_connection: Option<TcpStream>,
    //signal_connection: Option<TcpStream>,
    username: String,
    channelpool: Vec<Channel>, 
    rx: Option<Receiver<String>>,
}
impl Client {
    fn new() -> Client {
        Client { 
            id: String::new(),
            inited: false,
            connected: false,
            connected_to: Channel {
                id: String::new(),
                channel_name: String::new(),
                users: None,
                chat_msgs: None,
            },
            connection: None, 
            chat_connection: None,
            //signal_connection: None,
            username: String::new(),
            channelpool: Vec::new(), 
            rx: None,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct User {
    name: String,
    id: String,
 }
 
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct Channel {
    id: String,
    channel_name: String,
    users: Option<Vec<User>>,
    chat_msgs: Option<Vec<Message>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
   id: String,
   sender_id: String,
   sender: String,
   date: String,
   content: String,
}

#[derive(Default)]
struct App {
    //main_window_width: f32,
    //main_window_height: f32,
    client: Client,
    chat_text: String,
    username_text: String,
    join_channel_text: String,
    current_channel_text: String,
    connect_window: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn toggle_connection_window(&mut self) {
        if self.connect_window { 
            self.connect_window = false;
         } else { 
            self.connect_window = true;
        }
    }

    fn connection_window(&self) -> bool {
        self.connect_window
    } 
}

impl eframe::App for App {

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {  
        let (tx, rx) = mpsc::channel::<String>(); 

        if !self.client.inited { 
            self.client = init_user();
            self.client.rx.insert(rx);

            let signal_stream = TcpStream::connect("127.0.0.1:8083")
                .expect("Can't connect to main_stream"); 

            //Inting channelpool
            let conf = Ini::load_from_file("conf.ini").unwrap();
            let props = conf.section(Some("Channels"));
            for i in props {
                for (_, value) in i.iter() {
                    println!("value: {:?}", value);
                    submit_channel(&mut self.client, &value.to_string());
                }
            }

            //text fields
            self.username_text = self.client.username.clone();
            if !self.client.channelpool.is_empty() {
                self.current_channel_text = self.client.channelpool[0].channel_name.clone();
            }
                
            let stdin_channel = spawn_stdin_channel(signal_stream);
            std::thread::spawn(move || loop {
                match stdin_channel.try_recv() {
                    Ok(key) => {
                        tx.send(key).unwrap();
                    },
                    Err(TryRecvError::Empty) => { },
                    Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
                } 
                sleep(Duration::from_millis(100));
            });
        };
        
        if !self.client.chat_connection.is_none() {
            ctx.request_repaint();
            let received = self.client.rx.as_ref().unwrap().try_recv();
            match received {
                Ok(key) => {
                    if key.contains("UPDATEUSERS") {
                        if !self.client.connection.is_none() {
                            signal_server(self.client.connection.as_mut().unwrap(), "UPDATEUSERS");
                            update_channel_users(&mut self.client); 
                        }
                    } else if key.contains("UPDATECHAT") {    

                        signal_server(self.client.connection.as_mut().unwrap(), "UPDATECHAT");
                        let serialized = bincode::serialize(&self.client.connected_to).unwrap();
                        self.client.connection.as_ref().unwrap().write(&serialized).unwrap();
                        self.client.connection.as_ref().unwrap().flush().unwrap();

                        //lukee uudet viestit
                        let mut buf = vec![0; 100000];
                        self.client.connection.as_ref().unwrap().read(&mut buf).unwrap();
                        let mut deserialized: Vec<Message> = bincode::deserialize(&buf).unwrap();
                       // println!("de: {:#?}", deserialized);

                        //vertaa viimeisimpien viestejen id:itä ja että viestit ei ole None
                        if !deserialized.is_empty() {
                            if !self.client.connected_to.chat_msgs.as_ref().unwrap().is_empty() {
                                if !deserialized.last().unwrap().id.contains (
                                &self.client.connected_to.chat_msgs.as_ref().unwrap().last().unwrap().id) {
                                    self.client.connected_to.chat_msgs.as_mut().unwrap().append(&mut deserialized);
                                } 
                            } else  {
                                self.client.connected_to.chat_msgs.as_mut().unwrap().append(&mut deserialized);
                            }
                        }

                        // poistaa vanhimman viestin
                        println!("chat msgs len: {}", self.client.connected_to.chat_msgs.as_mut().unwrap().len());
                        if self.client.connected_to.chat_msgs.as_ref().unwrap().len() > CHAT_MAX_SIZE {
                            self.client.connected_to.chat_msgs.as_mut().unwrap().remove(0);
                            println!(" removed, chat msgs len: {}", self.client.connected_to.chat_msgs.as_mut().unwrap().len());
                        } 
                    }
                },
                Err(_) => { },
            }
        }
        
        // Top menubar
        egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {  
            menu::bar(ui, |ui| {
                if self.connection_window() {  
                    egui::Window::new("")
                    .title_bar(false)
                    .show(ctx, |ui| {
                        ui.allocate_ui_with_layout(Vec2 {x: 330.0, y: 6.0},
                            egui::Layout::right_to_left(), |ui| {
                                if ui.button("X").clicked() {
                                    self.toggle_connection_window();
                                }
                            });

                        ui.allocate_ui_with_layout(Vec2 {x: 400.0, y: 30.0},
                            egui::Layout::left_to_right(), |ui| {
                                ui.add(egui::Label::new("Username:"));
                                let res = ui.add(egui::TextEdit::singleline(&mut self.username_text)
                                    .desired_width(150.0));
                                if res.changed() {
                                    if self.username_text.len() > 16 {
                                        let s = self.username_text.split_at(16);
                                        self.username_text = s.0.to_string();
                                    }
                                    self.client.username = self.username_text.clone();
                                    println!("client.username: {}", self.client.username)
                                }
                            });

                        ui.allocate_ui_with_layout(Vec2 {x: 400.0, y: 30.0},
                            egui::Layout::left_to_right(), |ui| {
                                ui.add(egui::Label::new("Channel id:"));
                                ui.add(egui::TextEdit::singleline(&mut self.join_channel_text)
                                .desired_width(150.0)); 
                                if ui.button("Add Channel").clicked() {
                                    if submit_channel(&mut self.client, &self.join_channel_text) {
                                        for chnl in &self.client.channelpool { 
                                            if self.join_channel_text.contains(&chnl.id) {
                                                self.current_channel_text = chnl.channel_name.clone();
                                            }
                                        }
                                    }
                                    self.join_channel_text.clear();
                                }
                        });

                        ui.allocate_ui_with_layout( Vec2 {x: 400.0, y: 30.0},
                        egui::Layout::left_to_right(), |ui| {

                            ui.add(egui::Label::new("Channel list"));
                            egui::ComboBox::from_id_source("channelbox")
                                .width(235.0) 
                                .selected_text(&self.current_channel_text)
                                .show_ui(ui, |ui| {
                                    for chnl in &self.client.channelpool {
                                        ui.selectable_value(
                                        &mut self.current_channel_text,
                                        chnl.channel_name.to_string(),
                                        chnl.channel_name.to_string());
                                    };
                                });
                        });

                        ui.allocate_ui_with_layout( Vec2 {x: 0.0, y: 30.0},
                        egui::Layout::left_to_right(), |ui| {
                            if self.client.connected {
                                if ui.button(" Disconnect ").clicked() {
                                    signal_server(self.client.connection.as_mut().unwrap(), "DISCONNECT"); 
                                    disconnect(&mut self.client);
                                }
                            } else {
                                if ui.button("  Connect  ").clicked() {  
                                    let mut conf = Ini::load_from_file("conf.ini").unwrap();
                                    let mut value = self.client.username.clone();
                                    value.insert(0, '"');
                                    value.push('"');
                                    conf.with_section(Some("User"))
                                        .set("name", value);
                                    conf.write_to_file("conf.ini").unwrap();
    
                                    let mut channel = Channel {
                                        id: String::new(),
                                        channel_name: String::new(),
                                        users: None,
                                        chat_msgs: None
                                    };
                                    for c in &self.client.channelpool {
                                        if self.current_channel_text.contains(&c.channel_name) {
                                            channel = c.clone();
                                        }
                                    }
                                    connect_to_channel(&mut self.client, &mut channel);
                                    ctx.request_repaint(); 
                                    self.toggle_connection_window();
                                };
                            } 
                        });
                    });
                };

                //Top menubar "Connections" button
                ui.menu_button("Connections", |ui| {
                    //Connection button
                    if ui.button("Connect").clicked() {
                        ui.close_menu();
                        println!("{:#?}", self.client);
                        self.toggle_connection_window(); 
                    }
                    //Disconnection button
                    if ui.add_enabled(self.client.connected, egui::Button::new("Disconnect")).clicked() {
                        signal_server(self.client.connection.as_mut().unwrap(), "DISCONNECT"); 
                        disconnect(&mut self.client);
                        ui.close_menu();
                    }
                    //Quit button
                    if ui.button("Quit").clicked() {
                        signal_server(self.client.connection.as_mut().unwrap(), "DISCONNECT"); 
                        disconnect(&mut self.client);
                        eframe::Frame::quit(frame);
                    }
                });
            });
        });

        //Chat textedit
        egui::TopBottomPanel::bottom("bot_panel").show(ctx, |ui| {
            let max_text_len = 255;
            let text_len = format!("{}/{}",self.chat_text.len(), max_text_len);
            egui::Frame::none().inner_margin(Margin {
                left: 15.0,
                right: 15.0,
                top: 15.0,
                bottom: 0.0,
            })
            .show(ui, |ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut self.chat_text)
                .desired_width(f32::INFINITY));
                if self.chat_text.len() > max_text_len {
                    let s = self.chat_text.split_at(max_text_len);
                    self.chat_text = s.0.to_string();
                }
                if response.lost_focus()
                    && !self.client.connection.is_none() 
                    && self.client.connected != false
                    && ui.input().key_pressed(egui::Key::Enter) {  
                        response.request_focus();
                        signal_server(self.client.connection.as_mut().unwrap(), "CHATMSG"); 
                        let mut msg = self.chat_text.clone();
                        msg.push_str("\n");
                        msg = format!("{} {} {} {}", 
                            self.client.connected_to.id,
                            self.client.id,
                            self.client.username,
                            msg);
                        let msg = msg.as_bytes();
                        
                        self.client.connection.as_ref().unwrap()
                        .write(msg).expect("Cant write jostain syystä");
                        self.client.connection.as_ref().unwrap().flush().unwrap();
                        self.chat_text.clear();  
                }
            });
            ui.allocate_ui_with_layout(Vec2 {x: (ui.available_width() - 15.0), y: 25.0}, 
                egui::Layout::right_to_left(), |ui| {
                let color = || -> Color32 { 
                    let b = self.chat_text.len() == max_text_len;
                    if b { Color32::LIGHT_RED } else { Color32::BLACK} 
                };  
                ui.label(egui::RichText::new(text_len)
                .size(12.0)
                .color(color()));
             });    
        });

        egui::CentralPanel::default().show(ctx, |ui| { 
            egui::SidePanel::left("left_panel")
                .frame(egui::Frame::none()
                    .fill(Color32::WHITE))
                .resizable(false)
                .show_inside(ui, |ui| { 
                    for channel in &self.client.channelpool {
                        if channel.id.contains(&self.client.connected_to.id) {
                            if !channel.users.is_none() {
                                for users in channel.users.as_ref().unwrap() {
                                    if ui.link(users.name.clone()).clicked() {
                                        
                                    }
                                }
                            } else { break; }
                        }
                    }
            });
            let mut channel_name = "";
            if !self.client.connected_to.channel_name.is_empty() {
                channel_name = self.client.connected_to.channel_name.as_str();
            }
            ui.label(channel_name);

            egui::CentralPanel::default()
                .frame(egui::Frame::none() 
                .fill(Color32::WHITE))
                .show_inside(ui, |ui| { 
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_max_width(ui.available_width());
                        ui.set_max_height(f32::INFINITY);
                        if !self.client.connected_to.chat_msgs.is_none() {
                            for msg in self.client.connected_to.chat_msgs.as_ref().unwrap() {   
                                ui.horizontal_wrapped(|ui| { 
                                    ui.label(egui::RichText::new(&msg.sender)
                                        .size(16.0)
                                        .color(Color32::RED));
                                    let date = msg.date.clone();
                                    let mut split = date.split(' ');
                                    let date = format!("[ {} ]: ", split.next().unwrap());
                                    ui.label(egui::RichText::new(date)
                                        .color(Color32::GREEN)).on_hover_text(split.next().unwrap());
                                    ui.label(&msg.content);
                                });
                            };
                        };
                    });
                });
        });
    }

    fn on_exit(&mut self, _gl: &eframe::glow::Context) { 
        signal_server(self.client.connection.as_mut().unwrap(), "DISCONNECT"); 
        disconnect(&mut self.client);
        let mut conf = Ini::load_from_file("conf.ini").unwrap();
        let mut num = 1;
        conf.delete(Some("Channels"));
        for chnl in &self.client.channelpool {
            let name = format!("channel{}", num);
            conf.with_section(Some("Channels")).set(&name, &chnl.id);
            conf.write_to_file("conf.ini").unwrap();
            num += 1;
        }
        println!("Exiting client...");
    }
}

pub fn create_gui() {
    // let mut conf = Ini::load_from_file("conf.ini").unwrap();
    // let width: f32  = conf.with_section(Some("Window")).get("width").unwrap().parse().unwrap();
    // let height: f32  = conf.with_section(Some("Window")).get("height").unwrap().parse().unwrap();

    let mut native_options = eframe::NativeOptions::default(); 
    native_options.initial_window_size = Some(Vec2 {x: 600.0, y: 500.0});
    native_options.min_window_size = Some(Vec2 {x: 350.0, y: 200.0});
    eframe::run_native("app_name", native_options, Box::new(|cc| Box::new(App::new(cc))));
}

fn init_user() -> Client {
    let mut client = Client::new(); 
    let mut conf = Ini::load_from_file("conf.ini").unwrap();
    let id: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let main_stream = TcpStream::connect("127.0.0.1:8082")
        .expect("Can't connect to main_stream");
    client.connection = Some(main_stream); 
    
    println!("Connecting to {} from {}",
        client.connection.as_ref().unwrap().peer_addr().unwrap(),
        client.connection.as_ref().unwrap().local_addr().unwrap());

    client.id = id;
    client.username = conf.with_section(Some("User")).get("name").unwrap().to_string();
    client.inited = true;
    client
}

fn connect_to_channel(client: &mut Client, channel: &mut Channel) { 
    println!("channel {:#?}", channel);
    if client.connected {
        println!("Disconnecting...");
        disconnect(client);
    };

    signal_server(client.connection.as_mut().unwrap(), "INTCHAT");
    sleep(Duration::from_millis(100));
    println!("sending serialized...");
    let serialized = bincode::serialize(&channel).unwrap();
    client.connection.as_ref().unwrap().write(&serialized).unwrap();
    println!("sended: {:?} bytes", &serialized.capacity());

    signal_server(client.connection.as_mut().unwrap(), "CONNECT");
    let chat_stream = TcpStream::connect("127.0.0.1:8081").expect("Can't connect to main_stream"); 
    
    let buf = format!("{} {} {}{}", channel.id, client.username, client.id, "\n");
    let buf = buf.as_bytes();
    client.connection.as_ref().unwrap().write(buf).unwrap();
    client.connection.as_ref().unwrap().flush().unwrap();

    //Read deserialized chat messages
    let mut buf = vec![0; 100000];
    client.connection.as_ref().unwrap().read(&mut buf).unwrap();
    let deserialized: Vec<Message> = bincode::deserialize(&buf).unwrap(); 
    channel.chat_msgs.insert(deserialized);
    
    client.chat_connection = Some(chat_stream);
    client.connected = true;
    client.connected_to = channel.clone();

    println!("channel {:#?}", channel);
}

fn disconnect(client: &mut Client) { 
    let msg = client.connected_to.id.clone();
    signal_server(client.connection.as_mut().unwrap(), msg.as_str());
    let mut client = client; 
    client.connected = false;
    client.connected_to = Channel {
        id: String::new(),
        channel_name: String::new(),
        users: None,
        chat_msgs: None,
    };
    client.chat_connection.take();

    for channel in &mut client.channelpool {
        channel.users.take();
    }
    
}

fn submit_channel(client: &mut Client, channel_text: &String) -> bool { 
    let mut writer = BufWriter::new(client.connection.as_ref().unwrap().try_clone()
        .expect("cloning failed..."));
        
    let mut reader = BufReader::new(client.connection.as_ref().unwrap().try_clone()
        .expect("cloning failed..."));
    let mut msg = channel_text.clone();
    // Check for dublicate channels
    for chnl in &client.channelpool {
        if msg.contains(&chnl.id) {
            return false;
        }
    }
    signal_server(client.connection.as_mut().unwrap(), "ADDCHANNEL");
    msg.push_str("\n");
    let msg = msg.as_bytes(); 
    writer.write(msg).expect("Cant write to server");
    writer.flush().unwrap();

    let mut buf = vec![0; 255];
    reader.read(&mut buf).unwrap();

    let is_only_zeros = |buf: &[u8]| -> bool {
        let (prefix, aligned, suffix) = unsafe { buf.align_to::<u128>() };

        prefix.iter().all( |&x| x == 0 )
        && suffix.iter().all( |&x| x == 0 )
        && aligned.iter().all( |&x| x == 0 )
    };

    if !is_only_zeros(&buf) {
        let deserialized: Channel = bincode::deserialize(&buf).unwrap(); 
        client.channelpool.push(deserialized);
        true
    } else { 
        false
    }
}

fn update_channel_users(client: &mut Client) {   
    let mut reader = BufReader::new(client.connection
        .as_ref()
        .unwrap()
        .try_clone()
        .expect("cloning failed...")); 
    let msg = client.connected_to.id.clone();

    // Send the channel's id we want to update
    signal_server(client.connection.as_mut().unwrap(), msg.as_str());
    let mut buf = vec![0; 2048];
    reader.read(&mut buf).unwrap();
    let deserialized: Vec<User> = bincode::deserialize(&buf).unwrap();
    
    for c in &mut client.channelpool {
        if client.connected_to.id.contains(&c.id) {
            c.users.replace(deserialized);
            break;
        }
    }  
}

fn signal_server(stream: &mut TcpStream, signal: &str) {
    let mut writer = std::io::BufWriter::new(stream);
    let mut msg = String::from(signal);
    msg.push('\n');
    println!("sending signal.... = {:#?}", msg);
    let msg = msg.as_bytes();
    writer.write(msg).expect("Cant write to server");
    writer.flush().unwrap();
}

fn catch_signal(stream: &TcpStream) -> String { 
    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line.pop();
    line
 }

 fn spawn_stdin_channel(stream: TcpStream) -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || loop {
        let result = catch_signal(&stream);
        tx.send(result).unwrap();
    });
    rx
}
