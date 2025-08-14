use async_trait::async_trait;
use uuid::Uuid;

use crate::events::OrderEvent;

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn save_events(&self, events: Vec<OrderEvent>) -> Result<(), String>;
    async fn get_events(&self, order_id: Uuid) -> Result<Vec<OrderEvent>, String>;
    async fn get_all_events(&self) -> Result<Vec<OrderEvent>, String>;
}

pub struct InMemoryEventStore {
    events: dashmap::DashMap<Uuid, Vec<OrderEvent>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: dashmap::DashMap::new(),
        }
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn save_events(&self, events: Vec<OrderEvent>) -> Result<(), String> {
        for event in events {
            let order_id = match &event {
                OrderEvent::OrderPlaced(e) => e.order_id,
                OrderEvent::OrderCanceled(e) => e.order_id,
                OrderEvent::OrderUpdated(e) => e.order_id,
                OrderEvent::OrderMatched(e) => e.order_id,
                OrderEvent::OrderPartiallyFilled(e) => e.order_id,
                OrderEvent::OrderFilled(e) => e.order_id,
            };
            
            self.events
                .entry(order_id)
                .or_insert_with(Vec::new)
                .push(event);
        }
        Ok(())
    }

    async fn get_events(&self, order_id: Uuid) -> Result<Vec<OrderEvent>, String> {
        Ok(self.events
            .get(&order_id)
            .map(|events| events.clone())
            .unwrap_or_default())
    }

    async fn get_all_events(&self) -> Result<Vec<OrderEvent>, String> {
        Ok(self.events
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect())
    }
} 