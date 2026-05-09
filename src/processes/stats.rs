#[derive(Debug, Default, Clone)]
pub struct RunStats {
    pub torrents_total: usize,
    pub auto_management_enabled: usize,
    pub tracker_tags_added: usize,
    pub tracker_tags_removed: usize,
    pub share_limits_applied: usize,
    pub share_limits_cleaned_up: usize,
    pub share_limit_tags_added: usize,
    pub share_limit_tags_removed: usize,
    pub name_tags_added: usize,
    pub categories_changed: usize,
}

impl RunStats {
    pub fn log_summary(&self, dry_run: bool) {
        let suffix = if dry_run { " (dry-run)" } else { "" };
        log::info!(
            "==================== Summary{suffix} ====================\n  \
             Torrents inspected         : {}\n  \
             Auto-management enabled    : {}\n  \
             Tracker tags added         : {}\n  \
             Tracker tags removed       : {}\n  \
             Share limits applied       : {}\n  \
             Share limits cleaned up    : {}\n  \
             Share-limit tags added     : {}\n  \
             Share-limit tags removed   : {}\n  \
             Name tags added            : {}\n  \
             Categories changed         : {}",
            self.torrents_total,
            self.auto_management_enabled,
            self.tracker_tags_added,
            self.tracker_tags_removed,
            self.share_limits_applied,
            self.share_limits_cleaned_up,
            self.share_limit_tags_added,
            self.share_limit_tags_removed,
            self.name_tags_added,
            self.categories_changed,
        );
    }
}

pub fn log_section(name: &str) {
    log::info!("=========== {name} ===========");
}
