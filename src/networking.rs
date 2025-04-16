use crate::constants::{
    BROADCAST_ADDR, DISCOVERY_PORT, FIELD_SPLITTER, MSG_TYPE_CHAT, MSG_TYPE_DISCOVERY,
    MSG_TYPE_DISCOVERY_RESPONSE, RECV_BUFFER_SIZE,
};
use crate::message::Message;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::{HashSet, VecDeque};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, Receiver as MpscReceiver, Sender as MpscSender};
use tokio::time::sleep;

type MessageQueue = Arc<Mutex<VecDeque<Message>>>;
type PeerList = Arc<Mutex<HashSet<SocketAddr>>>;

pub struct Broadcaster {
    peers: PeerList,
    chat_port: u16,
    username: Arc<Mutex<String>>,
}

impl Clone for Broadcaster {
    fn clone(&self) -> Self {
        Self {
            peers: self.peers.clone(),
            chat_port: self.chat_port,
            username: self.username.clone(),
        }
    }
}

impl Broadcaster {
    pub fn new(chat_port: u16, username: String) -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashSet::new())),
            chat_port,
            username: Arc::new(Mutex::new(username)),
        }
    }

    #[allow(dead_code)]
    pub fn update_username(&self, new_username: String) {
        let mut username = self.username.lock().unwrap();
        *username = new_username;
    }

    pub fn get_peers(&self) -> PeerList {
        self.peers.clone()
    }

    pub async fn discover_peers(&self) -> io::Result<()> {
        // Create a socket for discovery
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_broadcast(true)?;
        socket.set_reuse_address(true)?;

        // Bind to any available port
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        socket.bind(&addr.into())?;

        // Convert to tokio UDP socket
        let discovery_socket = UdpSocket::from_std(socket.into())?;

        // Send discovery broadcast
        let username = self.username.lock().unwrap().clone();
        let discovery_msg = format!(
            "{}{}{}{}None",
            MSG_TYPE_DISCOVERY, FIELD_SPLITTER, username, FIELD_SPLITTER
        );
        
        // Broadcast to local subnet
        let broadcast_addr =
            SocketAddr::new(BROADCAST_ADDR.parse::<IpAddr>().unwrap(), DISCOVERY_PORT);
        
        // Send to local broadcast
        let _ = discovery_socket
            .send_to(discovery_msg.as_bytes(), broadcast_addr)
            .await;
            
        // Also try Tailscale subnet broadcast (100.100.100.100)
        // This is a special Tailscale address that may help with discovery
        if let Ok(tailscale_addr) = "100.100.100.100".parse::<IpAddr>() {
            let tailscale_broadcast = SocketAddr::new(tailscale_addr, DISCOVERY_PORT);
            let _ = discovery_socket
                .send_to(discovery_msg.as_bytes(), tailscale_broadcast)
                .await;
        }

        Ok(())
    }

    // This runs discovery periodically
    pub async fn discovery_service(broadcaster: Arc<Broadcaster>) -> io::Result<()> {
        loop {
            if let Err(e) = broadcaster.discover_peers().await {
                eprintln!("Peer discovery error: {}", e);
            }

            // Run discovery every 30 seconds
            sleep(Duration::from_secs(30)).await;
        }
    }

    pub async fn broadcast_message(&self, message: Message) -> io::Result<()> {
        // Create a socket for sending message
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_broadcast(true)?;
        socket.set_reuse_address(true)?;

        // Bind to any available port
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        socket.bind(&addr.into())?;

        // Convert to tokio UDP socket
        let udp_socket = UdpSocket::from_std(socket.into())?;

        // Format message with type
        let encoded_message = format!(
            "{}{}{}",
            MSG_TYPE_CHAT,
            FIELD_SPLITTER,
            message.encode_for_broadcast()
        );

        let peers = self.peers.lock().unwrap().clone();

        // Always send to known peers if we have any
        if !peers.is_empty() {
            // Send to each known peer
            for peer_addr in &peers {
                let target_addr = SocketAddr::new(peer_addr.ip(), self.chat_port);

                match udp_socket
                    .send_to(encoded_message.as_bytes(), target_addr)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to send to {}: {}", target_addr, e);
                    }
                }
            }
        }
        
        // Always try local broadcast (will work on local networks)
        let broadcast_addr =
            SocketAddr::new(BROADCAST_ADDR.parse::<IpAddr>().unwrap(), self.chat_port);
        let _ = udp_socket
            .send_to(encoded_message.as_bytes(), broadcast_addr)
            .await;
            
        // Also try Tailscale subnet broadcast (100.100.100.100)
        if let Ok(tailscale_addr) = "100.100.100.100".parse::<IpAddr>() {
            let tailscale_broadcast = SocketAddr::new(tailscale_addr, self.chat_port);
            let _ = udp_socket
                .send_to(encoded_message.as_bytes(), tailscale_broadcast)
                .await;
        }

        Ok(())
    }
}

pub struct Receiver {
    message_queue: MessageQueue,
    message_sender: MpscSender<Message>,
    message_receiver: Option<MpscReceiver<Message>>,
    peers: PeerList,
    username: Arc<Mutex<String>>,
}

impl Receiver {
    pub fn new(_chat_port: u16, username: String) -> Self {
        let message_queue = Arc::new(Mutex::new(VecDeque::new()));
        let (tx, rx) = mpsc::channel(100);

        Self {
            message_queue,
            message_sender: tx,
            message_receiver: Some(rx),
            peers: Arc::new(Mutex::new(HashSet::new())),
            username: Arc::new(Mutex::new(username)),
        }
    }

    pub fn get_peers(&self) -> PeerList {
        self.peers.clone()
    }

    #[allow(dead_code)]
    pub fn update_username(&self, new_username: String) {
        let mut username = self.username.lock().unwrap();
        *username = new_username;
    }

    pub fn parse_message(udp_data: &str) -> (String, String, String, String) {
        if udp_data.is_empty() {
            return (
                String::new(),
                "Unknown".to_string(),
                "".to_string(),
                "Unknown".to_string(),
            );
        }

        // Split by the field splitter
        let parts: Vec<&str> = udp_data.split(FIELD_SPLITTER).collect();

        if parts.len() < 2 {
            return (
                String::new(),
                "Unknown".to_string(),
                udp_data.to_string(),
                "Unknown".to_string(),
            );
        }

        let msg_type = parts[0].to_string();

        if parts.len() < 4 {
            return (
                msg_type,
                "Unknown".to_string(),
                parts[1..].join(FIELD_SPLITTER),
                "Unknown".to_string(),
            );
        }

        // For a standard chat message: MSG_TYPE, name, ip, content
        (
            msg_type,
            parts[1].to_string(),            // sender name
            parts[3..].join(FIELD_SPLITTER), // message content
            parts[2].to_string(),            // sender IP
        )
    }

    pub async fn handle_discovery(
        &self,
        socket: &UdpSocket,
        src: SocketAddr,
        data: &str,
    ) -> io::Result<()> {
        let (msg_type, sender_name, _content, _) = Self::parse_message(data);

        match msg_type.as_str() {
            MSG_TYPE_DISCOVERY => {
                // Someone is looking for peers, respond with our presence
                let username = self.username.lock().unwrap().clone();
                let response = format!(
                    "{}{}{}{}None",
                    MSG_TYPE_DISCOVERY_RESPONSE, FIELD_SPLITTER, username, FIELD_SPLITTER
                );
                socket.send_to(response.as_bytes(), src).await?;

                // Add this peer to our list
                self.peers.lock().unwrap().insert(src);
            }
            MSG_TYPE_DISCOVERY_RESPONSE => {
                // Someone responded to our discovery request, add them to peers
                self.peers.lock().unwrap().insert(src);
                println!("Discovered peer: {} ({})", sender_name, src.ip());
            }
            _ => {} // Ignore other message types
        }

        Ok(())
    }

    pub async fn listen_for_discovery(&self, discovery_port: u16) -> io::Result<()> {
        // Setup a socket with proper configuration using socket2
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        socket.set_broadcast(true)?;

        // Bind to the discovery port
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), discovery_port);
        socket.bind(&addr.into())?;

        // Convert to std socket
        let std_socket = socket.into();

        // Convert to tokio UDP socket
        let udp_socket = UdpSocket::from_std(std_socket)?;

        let mut buf = vec![0u8; RECV_BUFFER_SIZE];

        // Continuously listen for discovery messages
        loop {
            let (size, src) = udp_socket.recv_from(&mut buf).await?;
            let data = String::from_utf8_lossy(&buf[..size]).to_string();

            if let Err(e) = self.handle_discovery(&udp_socket, src, &data).await {
                eprintln!("Error handling discovery: {}", e);
            }
        }
    }

    pub async fn listen_for_messages(&mut self, chat_port: u16) -> io::Result<()> {
        // Setup a socket with proper configuration using socket2
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        socket.set_broadcast(true)?;

        // Bind to the chat port
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), chat_port);
        socket.bind(&addr.into())?;

        // Convert to std socket
        let std_socket = socket.into();

        // Convert to tokio UDP socket
        let udp_socket = UdpSocket::from_std(std_socket)?;

        let mut buf = vec![0u8; RECV_BUFFER_SIZE];
        let mut rx = self.message_receiver.take().unwrap();

        // Spawn a task to process messages from the channel and put them in the queue
        let queue = self.message_queue.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let mut queue = queue.lock().unwrap();
                queue.push_back(message);
            }
        });

        // Continuously listen for message UDP packets
        loop {
            let (size, src) = udp_socket.recv_from(&mut buf).await?;
            let data = String::from_utf8_lossy(&buf[..size]).to_string();

            let (msg_type, name, content, _) = Self::parse_message(&data);

            // Skip the message if it's not a chat message
            if msg_type != MSG_TYPE_CHAT {
                continue;
            }

            // Use the actual source IP address (from Tailscale or local network)
            let sender_ip = src.ip().to_string();

            // Create a new message and add it to our queue
            let message = Message::new(content, name, sender_ip);

            if let Err(e) = self.message_sender.send(message).await {
                eprintln!("Failed to add message to queue: {}", e);
            }

            // Add this peer to our known peers list
            self.peers.lock().unwrap().insert(src);
        }
    }

    pub fn get_queue_message(&self) -> Option<Message> {
        let mut queue = self.message_queue.lock().unwrap();
        queue.pop_front()
    }
}

impl Clone for Receiver {
    fn clone(&self) -> Self {
        // Create a new channel
        let (tx, rx) = mpsc::channel(100);

        Self {
            message_queue: self.message_queue.clone(),
            message_sender: tx,
            message_receiver: Some(rx),
            peers: self.peers.clone(),
            username: self.username.clone(),
        }
    }
}
