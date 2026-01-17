use iced::widget::Id;
use uuid::Uuid;

use crate::model::LauncherItem;

#[derive(Debug, Clone)]
pub struct CategoryList {
    pub items: Vec<LauncherItem>,
    pub selected_index: usize,
    pub scroll_id: Id,
}

impl CategoryList {
    pub fn new(items: Vec<LauncherItem>) -> Self {
        Self {
            items,
            selected_index: 0,
            scroll_id: Id::unique(),
        }
    }

    pub fn set_items(&mut self, items: Vec<LauncherItem>) {
        self.items = items;
        self.clamp_index();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected_index = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get_selected(&self) -> Option<&LauncherItem> {
        self.items.get(self.selected_index)
    }

    pub fn move_left(&mut self) -> bool {
        if !self.items.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
            return true;
        }
        false
    }

    pub fn move_right(&mut self) -> bool {
        if !self.items.is_empty() && self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
            return true;
        }
        false
    }

    pub fn update_item_by_id<F>(&mut self, id: Uuid, f: F)
    where
        F: FnOnce(&mut LauncherItem),
    {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            f(item);
        }
    }

    pub fn add_item(&mut self, item: LauncherItem) {
        self.items.push(item);
        self.sort_inplace();
        self.clamp_index();
    }

    pub fn remove_selected(&mut self) -> Option<LauncherItem> {
        if self.selected_index < self.items.len() {
            let removed = self.items.remove(self.selected_index);
            self.clamp_index();
            Some(removed)
        } else {
            None
        }
    }

    fn clamp_index(&mut self) {
        let len = self.items.len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len.saturating_sub(1);
        }
    }

    /// Sorts items by last_started timestamp (most recent first).
    /// Items that have never been launched are sorted alphabetically at the end.
    fn sort_items(items: &mut [LauncherItem]) {
        items.sort_by(|a, b| {
            match (a.last_started, b.last_started) {
                // Both have timestamps: sort by most recent first (descending)
                (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts),
                // Only a has timestamp: a comes first
                (Some(_), None) => std::cmp::Ordering::Less,
                // Only b has timestamp: b comes first
                (None, Some(_)) => std::cmp::Ordering::Greater,
                // Neither has timestamp: alphabetical fallback (case-insensitive)
                (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
    }

    pub fn sort_inplace(&mut self) {
        Self::sort_items(&mut self.items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LauncherAction, LauncherItem};

    fn item(name: &str) -> LauncherItem {
        LauncherItem {
            id: Uuid::new_v4(),
            name: name.to_string(),
            icon: None,
            system_icon: None,
            action: LauncherAction::Exit,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: None,
        }
    }

    fn item_with_timestamp(name: &str, timestamp: i64) -> LauncherItem {
        LauncherItem {
            id: Uuid::new_v4(),
            name: name.to_string(),
            icon: None,
            system_icon: None,
            action: LauncherAction::Exit,
            source_image_url: None,
            game_executable: None,
            launch_key: None,
            last_started: Some(timestamp),
        }
    }

    fn names(list: &CategoryList) -> Vec<&str> {
        list.items.iter().map(|i| i.name.as_str()).collect()
    }

    #[test]
    fn test_new_and_basic_operations() {
        let list = CategoryList::new(Vec::new());
        assert!(list.is_empty());
        assert_eq!(list.selected_index, 0);
        assert!(list.get_selected().is_none());

        let list = CategoryList::new(vec![item("A"), item("B")]);
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.get_selected().unwrap().name, "A");
    }

    #[test]
    fn test_move_left_right_boundaries() {
        let mut list = CategoryList::new(vec![item("A"), item("B"), item("C")]);

        // Can't move left from start
        assert!(!list.move_left());
        assert_eq!(list.selected_index, 0);

        // Move right twice
        assert!(list.move_right());
        assert!(list.move_right());
        assert_eq!(list.selected_index, 2);

        // Can't move right from end
        assert!(!list.move_right());
        assert_eq!(list.selected_index, 2);

        // Move left
        assert!(list.move_left());
        assert_eq!(list.selected_index, 1);

        // Empty list - no movement
        let mut empty = CategoryList::new(Vec::new());
        assert!(!empty.move_left());
        assert!(!empty.move_right());
    }

    #[test]
    fn test_remove_selected_clamps_index() {
        let mut list = CategoryList::new(vec![item("A"), item("B"), item("C")]);

        // Remove from middle - index stays, points to next item
        list.selected_index = 1;
        assert_eq!(list.remove_selected().unwrap().name, "B");
        assert_eq!(list.selected_index, 1);
        assert_eq!(list.get_selected().unwrap().name, "C");

        // Remove from end - index clamps down
        list.selected_index = 1;
        assert_eq!(list.remove_selected().unwrap().name, "C");
        assert_eq!(list.selected_index, 0);

        // Remove last item
        assert_eq!(list.remove_selected().unwrap().name, "A");
        assert!(list.is_empty());
        assert_eq!(list.selected_index, 0);

        // Remove from empty - returns None
        assert!(list.remove_selected().is_none());
    }

    #[test]
    fn test_add_item_sorts_and_set_items_clamps() {
        let mut list = CategoryList::new(vec![item("A"), item("C")]);
        list.add_item(item("B"));
        assert_eq!(names(&list), vec!["A", "B", "C"]);

        // set_items clamps out-of-bounds index
        list.selected_index = 2;
        list.set_items(vec![item("X")]);
        assert_eq!(list.selected_index, 0);

        // set_items preserves valid index
        list.set_items(vec![item("Y"), item("Z")]);
        list.selected_index = 1;
        list.set_items(vec![item("P"), item("Q")]);
        assert_eq!(list.selected_index, 1);
    }

    #[test]
    fn test_update_item_by_id() {
        let i = item("Original");
        let id = i.id;
        let mut list = CategoryList::new(vec![i]);

        list.update_item_by_id(id, |i| i.name = "Updated".to_string());
        assert_eq!(list.items[0].name, "Updated");

        // Non-existent ID does nothing
        list.update_item_by_id(Uuid::new_v4(), |i| i.name = "Nope".to_string());
        assert_eq!(list.items[0].name, "Updated");
    }

    #[test]
    fn test_sort_inplace_alphabetical_fallback() {
        // Items without timestamps should sort alphabetically
        let mut list = CategoryList::new(vec![item("C"), item("A"), item("B")]);
        list.sort_inplace();
        assert_eq!(names(&list), vec!["A", "B", "C"]);
    }

    #[test]
    fn test_sort_by_last_started() {
        // Items with timestamps should sort by most recent first
        let mut list = CategoryList::new(vec![
            item_with_timestamp("Old", 1000),
            item_with_timestamp("Newest", 3000),
            item_with_timestamp("Middle", 2000),
        ]);
        list.sort_inplace();
        assert_eq!(names(&list), vec!["Newest", "Middle", "Old"]);
    }

    #[test]
    fn test_sort_mixed_timestamps_and_no_timestamps() {
        // Items with timestamps come first (sorted by recency),
        // then items without timestamps (sorted alphabetically)
        let mut list = CategoryList::new(vec![
            item("Zebra"), // no timestamp
            item_with_timestamp("Game1", 1000),
            item("Apple"), // no timestamp
            item_with_timestamp("Game2", 2000),
        ]);
        list.sort_inplace();
        assert_eq!(names(&list), vec!["Game2", "Game1", "Apple", "Zebra"]);
    }

    #[test]
    fn test_sort_case_insensitive_alphabetical() {
        // Alphabetical fallback should be case-insensitive
        let mut list = CategoryList::new(vec![item("zebra"), item("Apple"), item("banana")]);
        list.sort_inplace();
        assert_eq!(names(&list), vec!["Apple", "banana", "zebra"]);
    }
}
