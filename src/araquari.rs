use base::config::keys;
use hbb_common::config;

const SERVER: &str = "jiraiya.araquari.sc.gov.br";
const PUBLIC_KEY: &str = "11CvwLZ0myJrVOg2amrOhqcKC0gZD1XIiFyL6QBnt+0=";

pub fn apply_defaults() {
    *config::APP_NAME.write().unwrap() = "AraquariDesk".to_owned();
    let mut settings = config::OVERWRITE_SETTINGS.write().unwrap();
    settings.insert(
        keys::OPTION_CUSTOM_RENDEZVOUS_SERVER.to_owned(),
        SERVER.to_owned(),
    );
    settings.insert(keys::OPTION_RELAY_SERVER.to_owned(), SERVER.to_owned());
    settings.insert(keys::OPTION_KEY.to_owned(), PUBLIC_KEY.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::config::Config;

    #[test]
    fn applies_municipal_server_configuration() {
        apply_defaults();
        assert_eq!(
            Config::get_option(keys::OPTION_CUSTOM_RENDEZVOUS_SERVER),
            SERVER
        );
        assert_eq!(Config::get_option(keys::OPTION_RELAY_SERVER), SERVER);
        assert_eq!(Config::get_option(keys::OPTION_KEY), PUBLIC_KEY);
        assert_eq!(config::APP_NAME.read().unwrap().as_str(), "AraquariDesk");
    }
}
