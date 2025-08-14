use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{OrderSide, OrderStatus, OrderType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderEvent {
    OrderPlaced(OrderPlacedEvent),
    OrderCanceled(OrderCanceledEvent),
    OrderUpdated(OrderUpdatedEvent),
    OrderMatched(OrderMatchedEvent),
    OrderPartiallyFilled(OrderPartiallyFilledEvent),
    OrderFilled(OrderFilledEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedEvent {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub status: OrderStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCanceledEvent {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdatedEvent {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub new_price: Option<Decimal>,
    pub new_quantity: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderMatchedEvent {
    pub order_id: Uuid,
    pub matched_order_id: Uuid,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: OrderSide,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPartiallyFilledEvent {
    pub order_id: Uuid,
    pub symbol: String,
    pub filled_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFilledEvent {
    pub order_id: Uuid,
    pub symbol: String,
    pub filled_quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}
