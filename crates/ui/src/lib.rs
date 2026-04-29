pub type WindowId = String;
pub type TabId = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceViewState {
    pub surface_id: String,
    pub width: u32,
    pub height: u32,
    pub focused: bool,
    pub frame_token: u64,
    pub frame_label: String,
    pub render_evidence: Option<String>,
    pub frame_buffer: Option<SurfaceFrameBuffer>,
    pub damage_events: u64,
    pub host_surface_failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceFrameBuffer {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabViewState {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub is_loading: bool,
    pub memory_indicator: Option<String>,
    pub failure_state: Option<String>,
    pub memory_attribution: Option<String>,
    pub status_text: String,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub surface: SurfaceViewState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserWindowModel {
    pub id: WindowId,
    pub tabs: Vec<TabViewState>,
    pub active_tab_id: Option<TabId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserCommand {
    Navigate { tab_id: TabId, url: String },
    Reload { tab_id: TabId },
    Back { tab_id: TabId },
    Forward { tab_id: TabId },
    ActivateTab { tab_id: TabId },
    CloseTab { tab_id: TabId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceMouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceInputEvent {
    PointerMove {
        x: i32,
        y: i32,
    },
    PointerButton {
        x: i32,
        y: i32,
        button: SurfaceMouseButton,
        pressed: bool,
        click_count: i32,
    },
    Wheel {
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    Key {
        key_code: i32,
        pressed: bool,
    },
    Text {
        text: String,
    },
    Focus {
        focused: bool,
    },
    Resize {
        width: u32,
        height: u32,
    },
}

impl BrowserWindowModel {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            active_tab_id: None,
        }
    }

    pub fn add_tab(&mut self, id: impl Into<String>, url: impl Into<String>) -> TabId {
        let tab_id = id.into();
        let url = url.into();
        self.tabs.push(TabViewState {
            id: tab_id.clone(),
            title: "New Tab".to_string(),
            url: url.clone(),
            is_loading: true,
            memory_indicator: None,
            failure_state: None,
            memory_attribution: None,
            status_text: format!("Preparing {url}"),
            can_go_back: false,
            can_go_forward: false,
            surface: SurfaceViewState {
                surface_id: format!("surface-{tab_id}"),
                width: 1280,
                height: 720,
                focused: false,
                frame_token: 0,
                frame_label: format!("Preparing {url}"),
                render_evidence: None,
                frame_buffer: None,
                damage_events: 0,
                host_surface_failure: None,
            },
        });
        self.active_tab_id = Some(tab_id.clone());
        tab_id
    }

    pub fn set_active_tab(&mut self, tab_id: &str) {
        if self.tabs.iter().any(|tab| tab.id == tab_id) {
            self.active_tab_id = Some(tab_id.to_string());
        }
        for tab in &mut self.tabs {
            tab.surface.focused = tab.id == tab_id;
        }
    }

    pub fn close_tab(&mut self, tab_id: &str) {
        let removed_index = self.tabs.iter().position(|tab| tab.id == tab_id);
        let was_active = self.active_tab_id.as_deref() == Some(tab_id);

        self.tabs.retain(|tab| tab.id != tab_id);

        self.active_tab_id = if self.tabs.is_empty() {
            None
        } else if was_active {
            let next_index = removed_index
                .unwrap_or(0)
                .min(self.tabs.len().saturating_sub(1));
            Some(self.tabs[next_index].id.clone())
        } else {
            self.active_tab_id
                .clone()
                .filter(|active_tab_id| self.tabs.iter().any(|tab| &tab.id == active_tab_id))
                .or_else(|| self.tabs.last().map(|tab| tab.id.clone()))
        };

        if let Some(active_id) = self.active_tab_id.clone() {
            self.set_active_tab(&active_id);
        }
    }

    pub fn update_from_engine(&mut self, next: TabViewState) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == next.id) {
            *tab = next;
        } else {
            self.tabs.push(next);
        }
    }

    pub fn set_memory_indicator(
        &mut self,
        tab_id: &str,
        memory_indicator: Option<String>,
        attribution: Option<String>,
    ) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.memory_indicator = memory_indicator;
            tab.memory_attribution = attribution;
        }
    }

    pub fn set_failure_state(&mut self, tab_id: &str, message: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.failure_state = message;
        }
    }

    pub fn set_surface_size(&mut self, tab_id: &str, width: u32, height: u32) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.surface.width = width;
            tab.surface.height = height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BrowserWindowModel, SurfaceViewState, TabViewState};

    fn updated_tab(id: &str) -> TabViewState {
        TabViewState {
            id: id.to_string(),
            title: "Updated".to_string(),
            url: "https://example.com/updated".to_string(),
            is_loading: false,
            memory_indicator: Some("memory warning".to_string()),
            failure_state: None,
            memory_attribution: Some("observed renderer metrics".to_string()),
            status_text: "Loaded updated page".to_string(),
            can_go_back: true,
            can_go_forward: false,
            surface: SurfaceViewState {
                surface_id: format!("surface-{id}"),
                width: 1440,
                height: 900,
                focused: true,
                frame_token: 3,
                frame_label: "Updated frame".to_string(),
                render_evidence: Some("test render evidence".to_string()),
                frame_buffer: None,
                damage_events: 1,
                host_surface_failure: None,
            },
        }
    }

    #[test]
    fn browser_window_tracks_active_tab() {
        let mut window = BrowserWindowModel::new("window-1");
        let tab = window.add_tab("tab-1", "https://webox.dev");
        window.update_from_engine(updated_tab(&tab));
        window.set_active_tab(&tab);
        assert_eq!(window.active_tab_id.as_deref(), Some("tab-1"));
        assert_eq!(window.tabs[0].title, "Updated");
        assert!(window.tabs[0].surface.focused);
    }

    #[test]
    fn closing_background_tab_keeps_current_active_tab() {
        let mut window = BrowserWindowModel::new("window-1");
        let first_tab = window.add_tab("tab-1", "https://webox.dev");
        let second_tab = window.add_tab("tab-2", "https://example.com");
        window.set_active_tab(&second_tab);

        window.close_tab(&first_tab);

        assert_eq!(window.active_tab_id.as_deref(), Some("tab-2"));
        assert_eq!(window.tabs.len(), 1);
    }

    #[test]
    fn closing_active_tab_falls_forward_to_remaining_neighbor() {
        let mut window = BrowserWindowModel::new("window-1");
        window.add_tab("tab-1", "https://webox.dev");
        let second_tab = window.add_tab("tab-2", "https://example.com");
        let _third_tab = window.add_tab("tab-3", "https://rust-lang.org");
        window.set_active_tab(&second_tab);

        window.close_tab(&second_tab);

        assert_eq!(window.active_tab_id.as_deref(), Some("tab-3"));
    }
}
