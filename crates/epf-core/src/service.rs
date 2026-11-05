use crate::{Engine, Error, OpenRequest, Result, Rule, RuleStatus, Snapshot, Store, UpnpRouter};
use std::path::Path;

pub fn snapshot(path: &Path) -> Result<Snapshot> {
    let (network, router) = UpnpRouter::discover();
    let store = Store::open(path)?;
    let (config, warning) = if let Some(router) = router {
        let mut engine = Engine::new(router, store)?;
        let warning = engine.refresh().err().map(|e| e.to_string());
        (engine.config().clone(), warning)
    } else {
        let mut config = store.load()?;
        for rule in &mut config.rules {
            rule.status = RuleStatus::Unknown;
        }
        (config, Some(network.message.clone()))
    };
    Ok(Snapshot {
        network,
        rules: config.rules,
        profiles: config.profiles,
        warning,
    })
}

pub fn open(path: &Path, request: OpenRequest, confirmed: bool) -> Result<Vec<Rule>> {
    request.validate()?;
    if !confirmed {
        return Err(Error::ConfirmationRequired);
    }
    let (network, router) = UpnpRouter::discover();
    let router = router.ok_or(Error::Network(network.message))?;
    Engine::new(router, Store::open(path)?)?.open(request, confirmed)
}

pub fn close(path: &Path, id: &str, confirmed: bool) -> Result<()> {
    if !confirmed {
        return Err(Error::ConfirmationRequired);
    }
    let (network, router) = UpnpRouter::discover();
    let router = router.ok_or(Error::Network(network.message))?;
    Engine::new(router, Store::open(path)?)?.close(id, confirmed)
}
