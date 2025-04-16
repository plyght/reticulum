pub const CHAT_PORT: u16 = 2223;
pub const DISCOVERY_PORT: u16 = 2224;
pub const RECV_BUFFER_SIZE: usize = 8192;

// Used for local network discovery via broadcast
pub const BROADCAST_ADDR: &str = "255.255.255.255";
// Multicast address for Tailscale discovery
pub const TAILSCALE_MULTICAST: &str = "100.100.100.100";

// Special message types for discovery
pub const MSG_TYPE_DISCOVERY: &str = "DISCOVER";
pub const MSG_TYPE_DISCOVERY_RESPONSE: &str = "DISCOVER_RESPONSE";
pub const MSG_TYPE_CHAT: &str = "CHAT";
pub const FIELD_SPLITTER: &str = "~";
pub const OUTBOUND_MESSAGE_REPORTED_IP: &str = "000.000.000.000";

// UI style stuff
pub const USER_INPUT_PROMPT: &str = "BROADCAST >>> ";
pub const USER_INPUT_PROMPT_LENGTH: usize = 14;
pub const START_MESSAGE_LINE: usize = 2;
pub const STATUS_BAR_LINE: usize = 1;

pub const LOGO_ASCII_ART: &str = " _______ _     _ ______  __   _ _______ _______       _    _  _____  _     _\n |______ |     | |_____] | \\  | |______    |           \\  /  |     |  \\___/ \n ______| |_____| |_____] |  \\_| |______    |    _____   \\/   |_____| _/   \\_";

pub const ONLINE_ASCII_ART: &str = "  _____  __   _        _____ __   _ _______\n |     | | \\  | |        |   | \\  | |______\n |_____| |  \\_| |_____ __|__ |  \\_| |______";

pub const DO_BULLSHIT_INTRO: bool = true;

// Common chat commands for tab completion
pub const COMMON_COMMANDS: [&str; 5] = ["/help", "/quit", "/clear", "/users", "/ping"];
