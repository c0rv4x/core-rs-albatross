use nimiq_handel::{contribution::AggregatableContribution, update::LevelUpdate};
use serde::{Deserialize, Serialize};

/// The serializable/deserializable representation of a LevelUpdate. It does away with the origin,
/// as that will be represented in the ValidatorMessage already.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "C: AggregatableContribution")]
pub struct SerializableLevelUpdate<C>
where
    C: AggregatableContribution,
{
    pub aggregate: C,
    pub individual: Option<C>,
    pub level: u8,
}

impl<C> SerializableLevelUpdate<C>
where
    C: AggregatableContribution,
{
    /// Given an origin, transforms this SerializableLevelUpdate into a LevelUpdate.
    pub fn into_level_update(self, origin: u16) -> LevelUpdate<C> {
        LevelUpdate {
            aggregate: self.aggregate,
            individual: self.individual,
            level: self.level,
            origin,
        }
    }
}

impl<C> From<LevelUpdate<C>> for SerializableLevelUpdate<C>
where
    C: AggregatableContribution,
{
    fn from(value: LevelUpdate<C>) -> Self {
        Self {
            aggregate: value.aggregate,
            individual: value.individual,
            level: value.level,
        }
    }
}
