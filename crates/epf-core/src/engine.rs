use crate::{
    now,
    router::{Mapping, Router},
    Config, Error, OpenRequest, Profile, Result, Rule, RuleStatus, Store,
};

pub struct Engine<R: Router> {
    pub router: R,
    store: Store,
    config: Config,
}

impl<R: Router> Engine<R> {
    pub fn new(router: R, store: Store) -> Result<Self> {
        let config = store.load()?;
        Ok(Self {
            router,
            store,
            config,
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn refresh(&mut self) -> Result<()> {
        let keys: Vec<_> = self
            .config
            .rules
            .iter()
            .filter(|r| !r.retired && r.router_id == self.router.id())
            .map(|r| (r.external_port, r.protocol))
            .collect();
        let mappings = match self.router.inspect(&keys) {
            Ok(mappings) => mappings,
            Err(e) => {
                for rule in &mut self.config.rules {
                    rule.status = RuleStatus::Unknown;
                }
                self.store.save(&self.config)?;
                return Err(e);
            }
        };
        let router_id = self.router.id();
        for rule in &mut self.config.rules {
            rule.status = if rule.retired {
                RuleStatus::Inactive
            } else if rule.router_id != router_id {
                RuleStatus::Unknown
            } else {
                observed_status(rule, &mappings)
            };
        }
        self.store.save(&self.config)
    }

    pub fn open(&mut self, request: OpenRequest, confirmed: bool) -> Result<Vec<Rule>> {
        request.validate()?;
        if !confirmed {
            return Err(Error::ConfirmationRequired);
        }
        let ip = request.destination_ip.unwrap_or(self.router.local_ip());
        if !self.router.accepts_destination(ip) {
            return Err(Error::Validation("Choose a destination on the detected local IPv4 subnet. VPN and remote destinations are not supported.".into()));
        }
        let protocols = request.protocol.expand();
        let keys: Vec<_> = protocols
            .iter()
            .map(|p| (request.external_port, *p))
            .collect();
        let mappings = self.router.inspect(&keys)?;
        for protocol in &protocols {
            if mappings
                .iter()
                .any(|m| m.external_port == request.external_port && m.protocol == *protocol)
            {
                return Err(Error::Conflict(format!("{} port {} already has a router rule. Choose another external port or close the existing EPF rule first.", protocol, request.external_port)));
            }
        }
        let rules: Vec<Rule> = protocols
            .into_iter()
            .map(|protocol| {
                let id = uuid::Uuid::new_v4().to_string();
                Rule {
                    description: format!("EPF:{id}"),
                    id,
                    name: request.name.trim().into(),
                    external_port: request.external_port,
                    internal_port: request.internal_port,
                    protocol,
                    destination_ip: ip,
                    lease_seconds: request.lease_seconds,
                    created_at: now(),
                    router_id: self.router.id(),
                    status: RuleStatus::Pending,
                    retired: false,
                }
            })
            .collect();
        let offset = self.config.rules.len();
        for previous in &mut self.config.rules {
            if rules.iter().any(|rule| {
                rule.router_id == previous.router_id
                    && rule.external_port == previous.external_port
                    && rule.protocol == previous.protocol
            }) {
                previous.retired = true;
                previous.status = RuleStatus::Inactive;
            }
        }
        self.config.rules.extend(rules.clone());
        // Journal every intended operation before the first router mutation.
        self.store.save(&self.config)?;
        for (index, rule) in rules.iter().enumerate() {
            if let Err(e) = self.router.add(rule) {
                for unattempted in self.config.rules.iter_mut().skip(offset + index + 1) {
                    unattempted.status = RuleStatus::Inactive;
                }
                self.store.save(&self.config)?;
                return Err(Error::Partial(format!("{e} Earlier protocols may already be active. The attempted rule is saved as pending; refresh to reconcile it before retrying.")));
            }
            self.config.rules[offset + index].status = RuleStatus::Active;
            // Persist each confirmed protocol separately. TCP and UDP can have different outcomes.
            self.store.save(&self.config)?;
        }
        Ok(self.config.rules[offset..].to_vec())
    }

    pub fn close(&mut self, id: &str, confirmed: bool) -> Result<()> {
        if !confirmed {
            return Err(Error::ConfirmationRequired);
        }
        let index = self
            .config
            .rules
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| {
                Error::Validation("This rule is not in EPF's saved configuration.".into())
            })?;
        let rule = self.config.rules[index].clone();
        if rule.retired {
            return Ok(());
        }
        if rule.router_id != self.router.id() {
            return Err(Error::Conflict("This rule belongs to a different router. Connect to the original network before closing it.".into()));
        }
        let mappings = self
            .router
            .inspect(&[(rule.external_port, rule.protocol)])?;
        let matching: Vec<_> = mappings.iter().filter(|m| m.matches(&rule)).collect();
        if !matching.is_empty() {
            if matching.len() != 1 || !matching[0].owned_by(&rule) {
                self.config.rules[index].status = RuleStatus::Conflict;
                self.store.save(&self.config)?;
                return Err(Error::Conflict(
                    "The router rule has changed or belongs to another app. EPF left it untouched."
                        .into(),
                ));
            }
            self.router.remove(&rule)?;
        }
        self.config.rules[index].status = RuleStatus::Inactive;
        self.config.rules[index].retired = true;
        self.store.save(&self.config)
    }
}

fn observed_status(rule: &Rule, mappings: &[Mapping]) -> RuleStatus {
    let matching: Vec<_> = mappings.iter().filter(|m| m.matches(rule)).collect();
    match matching.as_slice() {
        [] => RuleStatus::Inactive,
        [mapping] if mapping.owned_by(rule) => {
            if mapping.enabled {
                RuleStatus::Active
            } else {
                RuleStatus::Inactive
            }
        }
        _ => RuleStatus::Conflict,
    }
}

pub fn save_profile(store: &Store, request: OpenRequest) -> Result<Profile> {
    request.validate()?;
    let mut config = store.load()?;
    let profile = Profile {
        id: uuid::Uuid::new_v4().to_string(),
        request,
    };
    config.profiles.push(profile.clone());
    store.save(&config)?;
    Ok(profile)
}

pub fn delete_profile(store: &Store, id: &str) -> Result<()> {
    let mut config = store.load()?;
    config.profiles.retain(|p| p.id != id);
    store.save(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Protocol;
    use std::net::Ipv4Addr;

    #[derive(Default)]
    struct MockRouter {
        entries: Vec<Mapping>,
        adds: usize,
        removes: usize,
        fail_udp: bool,
        fail_read: bool,
        specific_only: bool,
    }
    impl Router for MockRouter {
        fn id(&self) -> String {
            "router-a".into()
        }
        fn local_ip(&self) -> Ipv4Addr {
            Ipv4Addr::new(192, 168, 1, 2)
        }
        fn accepts_destination(&self, ip: Ipv4Addr) -> bool {
            ip == self.local_ip()
        }
        fn mappings(&mut self) -> Result<Vec<Mapping>> {
            if self.fail_read {
                Err(Error::Network("timeout".into()))
            } else {
                Ok(self.entries.clone())
            }
        }
        fn inspect(&mut self, keys: &[(u16, Protocol)]) -> Result<Vec<Mapping>> {
            if self.specific_only {
                Ok(self
                    .entries
                    .iter()
                    .filter(|m| keys.contains(&(m.external_port, m.protocol)))
                    .cloned()
                    .collect())
            } else {
                self.mappings()
            }
        }
        fn add(&mut self, rule: &Rule) -> Result<()> {
            self.adds += 1;
            if self.fail_udp && rule.protocol == Protocol::Udp {
                return Err(Error::Network("UDP failed".into()));
            }
            self.entries.push(Mapping {
                external_port: rule.external_port,
                internal_port: rule.internal_port,
                protocol: rule.protocol,
                destination_ip: rule.destination_ip,
                description: rule.description.clone(),
                enabled: true,
                remote_host: String::new(),
            });
            Ok(())
        }
        fn remove(&mut self, rule: &Rule) -> Result<()> {
            self.removes += 1;
            self.entries.retain(|m| !m.matches(rule));
            Ok(())
        }
    }
    fn request() -> OpenRequest {
        OpenRequest {
            name: "Game".into(),
            external_port: 25565,
            internal_port: 25566,
            protocol: Protocol::Both,
            destination_ip: None,
            lease_seconds: 3600,
        }
    }
    fn engine(dir: &tempfile::TempDir) -> Engine<MockRouter> {
        Engine::new(
            MockRouter::default(),
            Store::open(dir.path().join("config.json")).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn confirmation_and_validation_precede_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        assert!(matches!(
            engine.open(request(), false),
            Err(Error::ConfirmationRequired)
        ));
        let mut invalid = request();
        invalid.external_port = 0;
        assert!(engine.open(invalid, true).is_err());
        let mut remote = request();
        remote.destination_ip = Some("8.8.8.8".parse().unwrap());
        assert!(engine.open(remote, true).is_err());
        assert_eq!(engine.router.adds, 0);
    }

    #[test]
    fn targeted_only_router_supports_lifecycle_and_still_refuses_foreign_rules() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        engine.router.specific_only = true;
        engine.router.fail_read = true;
        let rules = engine.open(request(), true).unwrap();
        engine.refresh().unwrap();
        assert!(engine
            .config
            .rules
            .iter()
            .all(|r| r.status == RuleStatus::Active));
        assert!(matches!(
            engine.open(request(), true),
            Err(Error::Conflict(_))
        ));
        engine.router.entries[0].description = "Another application".into();
        assert!(matches!(
            engine.close(&rules[0].id, true),
            Err(Error::Conflict(_))
        ));
        assert_eq!(engine.router.removes, 0);
        engine.close(&rules[1].id, true).unwrap();
        assert_eq!(engine.router.removes, 1);
    }
    #[test]
    fn opens_both_protocols_and_closes_only_owned_rules() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        let rules = engine.open(request(), true).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(engine.router.adds, 2);
        assert!(engine.open(request(), true).is_err());
        assert!(engine.close(&rules[0].id, false).is_err());
        engine.close(&rules[0].id, true).unwrap();
        assert_eq!(engine.router.entries.len(), 1);
        engine.router.entries[0].description = "Other application".into();
        assert!(engine.close(&rules[1].id, true).is_err());
        assert_eq!(engine.router.removes, 1);
    }
    #[test]
    fn partial_failure_is_persisted_and_refresh_recovers_pending_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        engine.router.fail_udp = true;
        assert!(matches!(
            engine.open(request(), true),
            Err(Error::Partial(_))
        ));
        let saved = engine.store.load().unwrap();
        assert_eq!(saved.rules[0].status, RuleStatus::Active);
        assert_eq!(saved.rules[1].status, RuleStatus::Pending);
        engine.refresh().unwrap();
        assert_eq!(engine.config.rules[1].status, RuleStatus::Inactive);
        engine.router.fail_read = true;
        assert!(engine.refresh().is_err());
        assert!(engine
            .config
            .rules
            .iter()
            .all(|r| r.status == RuleStatus::Unknown));
    }
    #[test]
    fn different_router_and_changed_destination_block_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        let rules = engine.open(request(), true).unwrap();
        engine.config.rules[0].router_id = "other-router".into();
        assert!(engine.close(&rules[0].id, true).is_err());
        engine.router.entries[1].destination_ip = "192.168.1.90".parse().unwrap();
        assert!(engine.close(&rules[1].id, true).is_err());
        assert_eq!(engine.router.removes, 0);
    }
    #[test]
    fn profiles_save_without_router_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(&dir);
        let profile = save_profile(&engine.store, request()).unwrap();
        assert_eq!(engine.store.load().unwrap().profiles.len(), 1);
        delete_profile(&engine.store, &profile.id).unwrap();
        assert!(engine.store.load().unwrap().profiles.is_empty());
        assert_eq!(engine.router.adds, 0);
    }

    #[test]
    fn retired_history_does_not_claim_a_reopened_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        let first = engine.open(request(), true).unwrap();
        for rule in first {
            engine.close(&rule.id, true).unwrap();
        }
        let second = engine.open(request(), true).unwrap();
        engine.refresh().unwrap();
        assert!(engine.config.rules[..2]
            .iter()
            .all(|r| r.retired && r.status == RuleStatus::Inactive));
        assert!(engine.config.rules[2..]
            .iter()
            .all(|r| !r.retired && r.status == RuleStatus::Active));
        assert_ne!(engine.config.rules[0].id, second[0].id);
        let deletes = engine.router.removes;
        let old_id = engine.config.rules[0].id.clone();
        engine.close(&old_id, true).unwrap();
        assert_eq!(engine.router.removes, deletes);
    }

    #[test]
    fn a_failed_journal_write_prevents_router_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = engine(&dir);
        // A directory where the config file should be makes atomic replacement fail.
        std::fs::create_dir(dir.path().join("config.json")).unwrap();
        assert!(engine.open(request(), true).is_err());
        assert_eq!(engine.router.adds, 0);
    }
}
