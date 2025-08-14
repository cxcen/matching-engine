use dashmap::DashMap;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::commands::{OrderCommand, PlaceOrderCommand};
use crate::event_store::EventStore;
use crate::events::{OrderEvent, OrderMatchedEvent, OrderPlacedEvent};
use crate::types::{Order, OrderBook, OrderSide, OrderType, Trade};

pub struct MatchingEngine {
    pub(crate) order_books: DashMap<String, OrderBook>,
    pub(crate) orders: DashMap<Uuid, Order>,
    pub(crate) trades: DashMap<Uuid, Trade>,
    event_store: Box<dyn EventStore>,
}

impl MatchingEngine {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            order_books: DashMap::new(),
            orders: DashMap::new(),
            trades: DashMap::new(),
            event_store,
        }
    }

    pub async fn handle_command(&self, command: OrderCommand) -> Result<Vec<OrderEvent>, String> {
        match command {
            OrderCommand::PlaceOrder(cmd) => self.handle_place_order(cmd).await,
            OrderCommand::CancelOrder(cmd) => self.handle_cancel_order(cmd).await,
        }
    }

    pub async fn handle_place_order(
        &self,
        cmd: PlaceOrderCommand,
    ) -> Result<Vec<OrderEvent>, String> {
        // Validate order
        self.validate_order(&cmd)?;

        // Create order
        let order = Order {
            id: cmd.order_id,
            user_id: cmd.user_id,
            symbol: cmd.symbol.clone(),
            order_type: cmd.order_type,
            side: cmd.side,
            price: cmd.price,
            quantity: cmd.quantity,
            filled_quantity: Decimal::ZERO,
            status: crate::types::OrderStatus::Pending,
            created_at: cmd.timestamp,
            updated_at: cmd.timestamp,
            iceberg_visible_quantity: cmd.iceberg_visible_quantity,
            stop_price: cmd.stop_price,
            trailing_stop_price: cmd.trailing_stop_price,
        };

        // Store order
        self.orders.insert(order.id, order.clone());

        // Create and save OrderPlaced event
        let placed_event = OrderPlacedEvent {
            order_id: order.id,
            user_id: order.user_id,
            symbol: order.symbol.clone(),
            order_type: order.order_type,
            side: order.side,
            price: order.price,
            quantity: order.quantity,
            status: order.status,
            timestamp: order.created_at,
        };

        let mut events = vec![OrderEvent::OrderPlaced(placed_event)];

        // Match order and generate events
        let trades = self.match_order(order).await?;
        for trade in trades {
            let matched_event = OrderMatchedEvent {
                order_id: trade.taker_order_id,
                matched_order_id: trade.maker_order_id,
                symbol: trade.symbol.clone(),
                price: trade.price,
                quantity: trade.quantity,
                side: trade.side,
                timestamp: trade.created_at,
            };
            events.push(OrderEvent::OrderMatched(matched_event));
        }

        // Save all events
        self.event_store.save_events(events.clone()).await?;

        Ok(events)
    }

    async fn handle_cancel_order(
        &self,
        _cmd: crate::commands::CancelOrderCommand,
    ) -> Result<Vec<OrderEvent>, String> {
        // Implementation for cancel order
        todo!()
    }

    pub(crate) fn validate_order(&self, cmd: &PlaceOrderCommand) -> Result<(), String> {
        match cmd.order_type {
            OrderType::Market => {
                if cmd.price.is_some() {
                    return Err("Market orders should not have a price".to_string());
                }
            }
            OrderType::Limit => {
                if cmd.price.is_none() {
                    return Err("Limit orders must have a price".to_string());
                }
            }
            OrderType::StopLoss | OrderType::TakeProfit => {
                if cmd.stop_price.is_none() {
                    return Err("Stop orders must have a stop price".to_string());
                }
            }
            OrderType::Iceberg => {
                if cmd.iceberg_visible_quantity.is_none() {
                    return Err("Iceberg orders must have a visible quantity".to_string());
                }
            }
            OrderType::TrailingStop => {
                if cmd.trailing_stop_price.is_none() {
                    return Err("Trailing stop orders must have a trailing stop price".to_string());
                }
            }
        }
        Ok(())
    }

    async fn match_order(&self, order: Order) -> Result<Vec<Trade>, String> {
        let mut trades = Vec::new();
        let mut remaining_quantity = order.quantity;

        match order.side {
            OrderSide::Buy => {
                // Match against asks (sell orders)
                if let Some(order_book) = self.order_books.get_mut(&order.symbol) {
                    while remaining_quantity > Decimal::ZERO {
                        if let Some(best_ask) = order_book.asks.first() {
                            match order.order_type {
                                OrderType::Market => {
                                    // Market orders match at any price
                                    let trade_quantity = remaining_quantity.min(best_ask.quantity);
                                    let trade = self.create_trade(
                                        &order,
                                        best_ask.price,
                                        trade_quantity,
                                        OrderSide::Buy,
                                    );
                                    trades.push(trade);
                                    remaining_quantity -= trade_quantity;
                                }
                                OrderType::Limit => {
                                    if let Some(price) = order.price {
                                        if price >= best_ask.price {
                                            let trade_quantity =
                                                remaining_quantity.min(best_ask.quantity);
                                            let trade = self.create_trade(
                                                &order,
                                                best_ask.price,
                                                trade_quantity,
                                                OrderSide::Buy,
                                            );
                                            trades.push(trade);
                                            remaining_quantity -= trade_quantity;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                _ => break,
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
            OrderSide::Sell => {
                // Match against bids (buy orders)
                if let Some(order_book) = self.order_books.get_mut(&order.symbol) {
                    while remaining_quantity > Decimal::ZERO {
                        if let Some(best_bid) = order_book.bids.first() {
                            match order.order_type {
                                OrderType::Market => {
                                    // Market orders match at any price
                                    let trade_quantity = remaining_quantity.min(best_bid.quantity);
                                    let trade = self.create_trade(
                                        &order,
                                        best_bid.price,
                                        trade_quantity,
                                        OrderSide::Sell,
                                    );
                                    trades.push(trade);
                                    remaining_quantity -= trade_quantity;
                                }
                                OrderType::Limit => {
                                    if let Some(price) = order.price {
                                        if price <= best_bid.price {
                                            let trade_quantity =
                                                remaining_quantity.min(best_bid.quantity);
                                            let trade = self.create_trade(
                                                &order,
                                                best_bid.price,
                                                trade_quantity,
                                                OrderSide::Sell,
                                            );
                                            trades.push(trade);
                                            remaining_quantity -= trade_quantity;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                _ => break,
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(trades)
    }

    fn create_trade(
        &self,
        order: &Order,
        price: Decimal,
        quantity: Decimal,
        side: OrderSide,
    ) -> Trade {
        let trade = Trade {
            id: Uuid::new_v4(),
            symbol: order.symbol.clone(),
            price,
            quantity,
            side,
            taker_order_id: order.id,
            maker_order_id: Uuid::new_v4(), // This should be the matched order's ID
            created_at: chrono::Utc::now(),
        };
        self.trades.insert(trade.id, trade.clone());
        trade
    }

    pub fn get_order_book(&self, symbol: &str) -> Option<OrderBook> {
        self.order_books.get(symbol).map(|ob| ob.clone())
    }

    pub fn get_order(&self, order_id: Uuid) -> Option<Order> {
        self.orders.get(&order_id).map(|o| o.clone())
    }

    pub fn get_trade(&self, trade_id: Uuid) -> Option<Trade> {
        self.trades.get(&trade_id).map(|t| t.clone())
    }
}
