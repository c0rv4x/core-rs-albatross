use std::collections::{HashMap, HashSet};

use libp2p::Multiaddr;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum NatStatus {
    Public,
    Private,
    #[default]
    Unknown,
}

#[derive(Default)]
pub(crate) struct NatState {
    confirmed_addresses: HashSet<Multiaddr>,
    listen_address_status: HashMap<Multiaddr, NatStatus>,
    status: NatStatus,
}

impl NatState {
    pub fn add_listen_address(&mut self, address: Multiaddr) {
        self.listen_address_status
            .insert(address, NatStatus::Unknown);
    }

    pub fn remove_listen_address(&mut self, address: &Multiaddr) {
        self.listen_address_status.remove(address);
        self.confirmed_addresses.remove(address);
        self.update_state();
    }

    pub fn set_listen_address_nat_status(&mut self, address: Multiaddr, nat_status: NatStatus) {
        if let Some(status) = self.listen_address_status.get_mut(&address) {
            *status = nat_status.clone();

            if nat_status == NatStatus::Public {
                self.confirmed_addresses.insert(address);
            } else {
                self.confirmed_addresses.remove(&address);
            }
            self.update_state();
        }
    }

    pub fn add_confirmed_address(&mut self, address: Multiaddr) {
        if let Some(address_status) = self.listen_address_status.get_mut(&address) {
            *address_status = NatStatus::Public;

            self.confirmed_addresses.insert(address);
            self.update_state();
        }
    }

    pub fn remove_confirmed_address(&mut self, address: &Multiaddr) {
        if let Some(address_status) = self.listen_address_status.get_mut(address) {
            *address_status = NatStatus::Private;

            self.confirmed_addresses.remove(address);
            self.update_state();
        }
    }

    fn update_state(&mut self) {
        let old_nat_status = self.status.clone();

        if !self.confirmed_addresses.is_empty() {
            self.status = NatStatus::Public;
        } else if self
            .listen_address_status
            .iter()
            .all(|(_, status)| *status == NatStatus::Private)
        {
            self.status = NatStatus::Private;
        } else {
            self.status = NatStatus::Unknown
        }

        Self::handle_new_status(&old_nat_status, &self.status);
    }

    fn handle_new_status(old_nat_status: &NatStatus, new_nat_status: &NatStatus) {
        if old_nat_status == new_nat_status {
            return;
        }

        if *new_nat_status == NatStatus::Private {
            log::warn!("Couldn't detect a public reachable address. Validator network operations won't be possible");
            log::warn!("You may need to find a relay to enable validator network operations");
        } else if *new_nat_status == NatStatus::Public {
            log::info!(
                ?old_nat_status,
                ?new_nat_status,
                "NAT status changed and detected public reachable address. Validator network operations will be possible"
            );
        }
    }
}
