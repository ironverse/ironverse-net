use bevy::{prelude::*, utils::HashMap};
use bevy_egui::{egui, EguiContext};
use std::collections::VecDeque;

const MAX_LINES: usize = 100;

#[derive(Clone, Debug)]
pub struct ChatEvent {
    pub is_outgoing: bool,
    pub sender: String,
    pub message: String,
}

#[derive(Default)]
pub struct ChatResource {
    pub chat_history: VecDeque<ChatEvent>,
    pub chat_input_text: String,
    pub name_map: HashMap<String, String>,
}

pub fn chat_event_reader(mut local: ResMut<ChatResource>, mut chat_reader: EventReader<ChatEvent>) {
    for event in chat_reader.iter() {
        if event.message.starts_with("/name ") && event.message.len() > 6 {
            let old_name = get_username(&local.name_map, event.sender.clone());
            let new_name = event.message[6..].to_string();
            if !event.is_outgoing {
                local.name_map.insert(event.sender.clone(), new_name.clone());
            }
            local.chat_history.add_line(ChatEvent { 
                is_outgoing: event.is_outgoing, 
                sender: old_name, 
                message: format!("changed their name to {}", new_name),
            });
        } else if event.message.starts_with("/pub ") && event.message.len() > 5 {
            let name = get_username(&local.name_map, event.sender.clone());
            local.chat_history.add_line(ChatEvent { 
                is_outgoing: event.is_outgoing, 
                sender: name, 
                message: event.message[5..].to_string(),
            });
        } else {
            local.chat_history.add_line(event.clone());
        }
    }
}
fn get_username(name_map: &HashMap<String, String>, sender: String) -> String {
    let name = match name_map.get(&sender) {
        Some(name) => name.to_string(),
        None => String::from(""),
    };
    let id = if sender.len() >= 4 {
        format!("{}", sender[sender.len()-4..].to_string())
    } else {
        sender
    };
    if name.len() == 0 {
        format!("{}", id)
    } else {
        format!("{}/{}", name, id)
    }
}

pub fn chat_window(mut local: ResMut<ChatResource>, mut egui_context: ResMut<EguiContext>, mut chat_writer: EventWriter<ChatEvent>) {
    egui::Window::new("Chat Window").show(egui_context.ctx_mut(), |ui| {
        egui::ScrollArea::new([false, true]).auto_shrink([false, false]).max_height(500.0).stick_to_bottom().show(ui, |ui|{
            for mut line in local.chat_history.to_string_vec() {
                ui.add(egui::TextEdit::multiline(&mut line).margin(egui::Vec2{x: 4.0, y: 0.0}).desired_rows(1).desired_width(ui.available_width()).frame(false));
            }
        });

        let chat_input_response = ui.add(egui::TextEdit::singleline(&mut local.chat_input_text).desired_width(ui.available_width()));
        
        if chat_input_response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
            if !local.chat_input_text.starts_with("/") {
                local.chat_input_text = format!("/pub {}", local.chat_input_text.to_string())
            }
            chat_writer.send(ChatEvent{
                is_outgoing: true,
                sender: String::from("me"),
                message: local.chat_input_text.clone()
            });
            local.chat_input_text = String::new();
            chat_input_response.request_focus();
        }
    });
    egui::Window::new("Help Window").show(egui_context.ctx_mut(), |ui| {
        let lines = vec![
            "Commands:",
            "",
            "Set your username:",
            "/name myusername",
            "",
            "Add a peer manually using ip and port:",
            "/dial /ip4/127.0.0.1/tcp/49987",
            "",
            "Subscribe to a topic:",
            "/sub topicname",
            "",
            "Unsubscribe from a topic:",
            "/unsub topicname",
        ];
        ui.label(lines.join("\n"));
    });
}
trait ChatHistory{
    fn add_line(&mut self, line: ChatEvent);
    fn to_string_vec(&self) -> Vec<String>;
}

impl ChatHistory for VecDeque<ChatEvent> {
    fn add_line(&mut self, line: ChatEvent) {
        self.push_back(line);
        if self.len() > MAX_LINES {
            self.pop_front();
        }
    }
    
    fn to_string_vec(&self) -> Vec<String> {
        return self.into_iter().map(|a| a.to_string()).collect();
    }
}

impl ChatEvent {
    pub fn to_string(&self) -> String {
        format!("{}: {}", self.sender.clone(), self.message.clone())
    }
    pub fn new_system_msg(message: &str) -> ChatEvent {
        ChatEvent{
            is_outgoing: false,
            sender: String::from("System"),
            message: String::from(message),
        }
    }
}