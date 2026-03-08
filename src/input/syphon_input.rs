//! Syphon Input (macOS)
//!
//! Receives video frames from Syphon servers using syphon-core crate.

use std::time::Instant;

/// Re-export from syphon-core
pub use syphon_core::ServerInfo as SyphonServerInfo;

/// A received Syphon frame
pub struct SyphonFrame {
    pub width: u32,
    pub height: u32,
    /// BGRA pixel data (native macOS format)
    pub data: Vec<u8>,
    pub timestamp: Instant,
}

/// Syphon input receiver
pub struct SyphonInputReceiver {
    #[cfg(target_os = "macos")]
    client: Option<syphon_core::SyphonClient>,
    server_name: Option<String>,
    resolution: (u32, u32),
}

impl SyphonInputReceiver {
    /// Create a new Syphon input receiver
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            client: None,
            server_name: None,
            resolution: (1920, 1080),
        }
    }
    
    /// Check if Syphon is available
    pub fn is_available() -> bool {
        syphon_core::is_available()
    }
    
    /// Connect to a Syphon server by name
    pub fn connect(&mut self, server_name: impl Into<String>) -> anyhow::Result<()> {
        let server_name = server_name.into();
        
        if self.is_connected() {
            self.disconnect();
        }
        
        log::info!("[Syphon Input] Connecting to: {}", server_name);
        
        #[cfg(target_os = "macos")]
        {
            let client = syphon_core::SyphonClient::connect(&server_name)
                .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;
            
            self.client = Some(client);
        }
        
        self.server_name = Some(server_name);
        Ok(())
    }
    
    /// Try to receive a new frame
    pub fn try_receive(&mut self) -> Option<SyphonFrame> {
        #[cfg(target_os = "macos")]
        {
            let client = self.client.as_ref()?;
            
            match client.try_receive() {
                Ok(Some(mut frame)) => {
                    self.resolution = (frame.width, frame.height);
                    
                    match frame.to_vec() {
                        Ok(data) => {
                            return Some(SyphonFrame {
                                width: frame.width,
                                height: frame.height,
                                data,
                                timestamp: Instant::now(),
                            });
                        }
                        Err(e) => {
                            log::warn!("[Syphon Input] Failed to read frame: {}", e);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("[Syphon Input] Error: {}", e);
                }
            }
        }
        
        None
    }
    
    /// Disconnect from current server
    pub fn disconnect(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.client = None;
        }
        self.server_name = None;
    }
    
    /// Check if connected
    pub fn is_connected(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.client.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    
    /// Get current resolution
    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }
    
    /// Get connected server name
    pub fn server_name(&self) -> Option<&str> {
        self.server_name.as_deref()
    }
}

impl Default for SyphonInputReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SyphonInputReceiver {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Syphon server discovery
pub struct SyphonDiscovery;

impl SyphonDiscovery {
    /// Create new discovery
    pub fn new() -> Self {
        Self
    }
    
    /// Discover available Syphon servers
    pub fn discover_servers(&self) -> Vec<SyphonServerInfo> {
        syphon_core::SyphonServerDirectory::servers()
    }
    
    /// Check if a server exists
    pub fn is_server_available(&self, name: &str) -> bool {
        syphon_core::SyphonServerDirectory::server_exists(name)
    }
}

impl Default for SyphonDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience integration struct
pub struct SyphonInputIntegration {
    receiver: Option<SyphonInputReceiver>,
    discovery: SyphonDiscovery,
    cached_servers: Vec<SyphonServerInfo>,
    last_discovery: Option<Instant>,
}

impl SyphonInputIntegration {
    /// Create new integration
    pub fn new() -> Self {
        Self {
            receiver: None,
            discovery: SyphonDiscovery::new(),
            cached_servers: Vec::new(),
            last_discovery: None,
        }
    }
    
    /// Check if Syphon is available
    pub fn is_available() -> bool {
        SyphonInputReceiver::is_available()
    }
    
    /// Refresh server list
    pub fn refresh_servers(&mut self) {
        self.cached_servers = self.discovery.discover_servers();
        self.last_discovery = Some(Instant::now());
    }
    
    /// Get cached servers
    pub fn servers(&self) -> &[SyphonServerInfo] {
        &self.cached_servers
    }
    
    /// Connect to a server
    pub fn connect(&mut self, server_name: &str) -> anyhow::Result<()> {
        self.disconnect();
        
        let mut receiver = SyphonInputReceiver::new();
        receiver.connect(server_name)?;
        self.receiver = Some(receiver);
        
        Ok(())
    }
    
    /// Disconnect
    pub fn disconnect(&mut self) {
        self.receiver = None;
    }
    
    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.receiver.as_ref().map_or(false, |r| r.is_connected())
    }
    
    /// Get latest frame
    pub fn get_frame(&mut self) -> Option<SyphonFrame> {
        self.receiver.as_mut()?.try_receive()
    }
    
    /// Update (refresh discovery periodically)
    pub fn update(&mut self) {
        if self.last_discovery.map_or(true, |t| t.elapsed().as_secs() > 5) {
            self.refresh_servers();
        }
    }
}

impl Default for SyphonInputIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_receiver_creation() {
        let receiver = SyphonInputReceiver::new();
        assert!(!receiver.is_connected());
    }
}
