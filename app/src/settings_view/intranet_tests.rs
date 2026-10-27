use super::SettingsSection;

#[test]
fn intranet_settings_start_on_local_appearance_page() {
    assert_eq!(SettingsSection::default(), SettingsSection::Appearance);
}

#[test]
fn intranet_hides_service_settings_and_redirects_restore() {
    for section in [
        SettingsSection::Account,
        SettingsSection::BillingAndUsage,
        SettingsSection::Teams,
        SettingsSection::Referrals,
        SettingsSection::SharedBlocks,
        SettingsSection::WarpDrive,
        SettingsSection::WarpAgent,
        SettingsSection::AgentProfiles,
        SettingsSection::AgentMCPServers,
        SettingsSection::Knowledge,
        SettingsSection::ThirdPartyCLIAgents,
        SettingsSection::CodeIndexing,
        SettingsSection::CloudEnvironments,
        SettingsSection::WarpCloudAgentAPIKeys,
        SettingsSection::Privacy,
        SettingsSection::Scripting,
    ] {
        assert!(!section.is_available_in_intranet());
        assert_eq!(section.available_or_default(), SettingsSection::Appearance);
    }
}

#[test]
fn intranet_preserves_local_terminal_and_editor_settings() {
    for section in [
        SettingsSection::About,
        SettingsSection::Appearance,
        SettingsSection::Features,
        SettingsSection::Keybindings,
        SettingsSection::Warpify,
        SettingsSection::EditorAndCodeReview,
    ] {
        assert!(section.is_available_in_intranet());
        assert_eq!(section.available_or_default(), section);
    }
}
