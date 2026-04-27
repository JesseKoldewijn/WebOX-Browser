pub type WindowId = String;
pub type TabId = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabViewState {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub is_loading: bool,
    pub memory_indicator: Option<String>,
    pub failure_state: Option<String>,
    history: Vec<String>,
    history_index: usize,
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
            history: vec![url],
            history_index: 0,
        });
        self.active_tab_id = Some(tab_id.clone());
        tab_id
    }

    pub fn set_active_tab(&mut self, tab_id: &str) {
        if self.tabs.iter().any(|tab| tab.id == tab_id) {
            self.active_tab_id = Some(tab_id.to_string());
        }
    }

    pub fn navigate_tab(&mut self, tab_id: &str, url: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.url = url.to_string();
            tab.is_loading = true;
            tab.history.truncate(tab.history_index + 1);
            tab.history.push(url.to_string());
            tab.history_index = tab.history.len() - 1;
        }
    }

    pub fn finish_loading(&mut self, tab_id: &str, title: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.is_loading = false;
            tab.title = title.to_string();
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
    }

    pub fn go_back(&mut self, tab_id: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            if tab.history_index > 0 {
                tab.history_index -= 1;
                tab.url = tab.history[tab.history_index].clone();
            }
        }
    }

    pub fn go_forward(&mut self, tab_id: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            if tab.history_index + 1 < tab.history.len() {
                tab.history_index += 1;
                tab.url = tab.history[tab.history_index].clone();
            }
        }
    }

    pub fn set_memory_indicator(&mut self, tab_id: &str, memory_indicator: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.memory_indicator = memory_indicator;
        }
    }

    pub fn set_failure_state(&mut self, tab_id: &str, message: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.failure_state = message;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BrowserWindowModel;

    #[test]
    fn browser_window_tracks_active_tab() {
        let mut window = BrowserWindowModel::new("window-1");
        let tab = window.add_tab("tab-1", "https://webox.dev");
        window.finish_loading(&tab, "webox");
        assert_eq!(window.active_tab_id.as_deref(), Some("tab-1"));
        assert_eq!(window.tabs[0].title, "webox");
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
