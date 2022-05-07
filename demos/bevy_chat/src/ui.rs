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
        let mut event = event.clone();
        if event.message.starts_with("/name ") {
            let name = event.message[6..].to_string();
            event.message = format!("changed their name to {}", name);
            event.sender = get_username(&local.name_map, event.sender);
            local.name_map.insert(event.sender.clone(), name);
            local.chat_history.add_line(event);
        } else if event.message.starts_with("/pub ") {
            event.message = event.message[5..].to_string();
            event.sender = get_username(&local.name_map, event.sender);
            local.chat_history.add_line(event);
        } else {
            local.chat_history.add_line(event);
        }
    }
}
fn get_username(name_map: &HashMap<String, String>, mut sender: String) -> String {
    if sender.len() >= 4 {
        sender = sender[sender.len()-4..].to_string()
    }
    let name = match name_map.get(&sender) {
        Some(name) => name.to_string(),
        None => String::from(""),
    };
    format!("{}#{}", name, sender)
}

pub fn chat_window(mut local: ResMut<ChatResource>, mut egui_context: ResMut<EguiContext>, keyboard_input: Res<Input<KeyCode>>, mut chat_writer: EventWriter<ChatEvent>) {
    egui::Window::new("Chat Window").show(egui_context.ctx_mut(), |ui| {
        egui::ScrollArea::new([false, true]).auto_shrink([false, false]).max_height(600.0).stick_to_bottom().show(ui, |ui|{
            ui.label(local.chat_history.to_string());
        });

        let chat_input_response = ui.add(egui::TextEdit::singleline(&mut local.chat_input_text).desired_width(ui.available_width()));
        chat_input_response.request_focus();

        if chat_input_response.has_focus() && keyboard_input.just_pressed(KeyCode::Return) {
            if !local.chat_input_text.starts_with("/") {
                local.chat_input_text = format!("/pub {}", local.chat_input_text.to_string())
            }
            chat_writer.send(ChatEvent{
                is_outgoing: true,
                sender: String::from("me"),
                message: local.chat_input_text.clone()
            });
            local.chat_input_text = String::new();
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
    fn to_string(&self) -> String;
}

impl ChatHistory for VecDeque<ChatEvent> {
    fn add_line(&mut self, line: ChatEvent) {
        self.push_back(line);
        if self.len() > MAX_LINES {
            self.pop_front();
        }
    }
    
    fn to_string(&self) -> String {
        let mut history_text = String::new();
        if self.len() > 0 {
            history_text = self[0].to_string();
            for i in 1..self.len() {
                history_text = format!("{}\n{}", history_text, self[i].to_string());
            }
        }
        return history_text;
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