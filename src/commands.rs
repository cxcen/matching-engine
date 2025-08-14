use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{OrderSide, OrderType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    PlaceOrder(PlaceOrderCommand),
    CancelOrder(CancelOrderCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderCommand {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub iceberg_visible_quantity: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trailing_stop_price: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderCommand {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
}
