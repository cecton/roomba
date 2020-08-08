use tui::widgets::ListState;

pub struct StatefulList {
    pub state: ListState,
    pub items: Vec<(String, bool)>,
}

impl StatefulList {
    pub fn with_items(items: Vec<(String, bool)>) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn select(&mut self) {
        if let Some(i) = self.state.selected() {
            self.items[i].1 ^= true;
            self.items.sort_by_key(|x| !x.1);
        }
    }

    pub fn move_up(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 && self.items[i].1 && self.items[i - 1].1 {
                let elem = self.items[i].clone();
                self.items[i] = self.items[i-1].clone();
                self.items[i - 1] = elem;
                self.state.select(Some(i-1));
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.items.len() - 1 && self.items[i].1 && self.items[i + 1].1 {
                let elem = self.items[i].clone();
                self.items[i] = self.items[i+1].clone();
                self.items[i + 1] = elem;
                self.state.select(Some(i+1));
            }
        }
    }
}
