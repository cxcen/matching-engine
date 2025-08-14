use rust_decimal::Decimal;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use uuid::Uuid;

use crate::types::{Order, OrderBookEntry, OrderSide};

const MAX_LEVEL: usize = 32;

type SkipNode = Rc<RefCell<Node>>;

fn new_skip_node(price: Decimal, level: usize) -> SkipNode {
    Rc::new(RefCell::new(Node::new(price, level)))
}

#[derive(Debug, Clone)]
struct Node {
    price: Decimal,
    orders: Vec<Order>,
    next: Vec<Option<SkipNode>>,
}

impl Node {
    fn new(price: Decimal, level: usize) -> Self {
        Self {
            price,
            orders: Vec::new(),
            next: vec![None; level],
        }
    }
}

#[derive(Debug)]
pub struct SkipListOrderBook {
    head: SkipNode,
    level: usize,
    size: usize,
    price_map: HashMap<Decimal, Vec<Order>>,
}

impl SkipListOrderBook {
    pub fn new() -> Self {
        Self {
            head: new_skip_node(Decimal::MIN, MAX_LEVEL),
            level: 1,
            size: 0,
            price_map: HashMap::new(),
        }
    }

    fn random_level() -> usize {
        let mut level = 1;
        while level < MAX_LEVEL && rand::random::<f64>() < 0.5 {
            level += 1;
        }
        level
    }

    pub fn add_order(&mut self, order: Order) {
        let price = order.price.unwrap_or(Decimal::MAX);
        self.price_map
            .entry(price)
            .or_insert_with(Vec::new)
            .push(order.clone());

        let mut current = Some(self.head.clone());
        let mut update = vec![self.head.clone(); MAX_LEVEL];
        let mut level = self.level;

        while level > 0 {
            level -= 1;
            while let Some(node) = current.clone() {
                if let Some(next) = node.borrow().next[level].clone() {
                    if next.borrow().price > price {
                        break;
                    }
                    current = Some(next);
                }
            }
            
            update[level] = current.take().unwrap();
        }
        

        let new_level = Self::random_level();
        if new_level > self.level {
            for i in self.level..new_level {
                update[i] = self.head.clone();
            }
            self.level = new_level;
        }

        let new_node = new_skip_node(price, new_level);
        for i in 0..new_level {
            new_node.borrow_mut().next[i] = update[i].borrow_mut().next[i].take();
            update[i].borrow_mut().next[i] = Some(new_node.clone());
        }

        self.size += 1;
    }

    pub fn remove_order(&mut self, order_id: Uuid, price: Decimal) -> Option<Order> {
        if let Some(orders) = self.price_map.get_mut(&price) {
            if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                let order = orders.remove(pos);
                if orders.is_empty() {
                    self.price_map.remove(&price);
                }
                self.size -= 1;
                return Some(order);
            }
        }
        None
    }

    pub fn get_best_price(&self, side: OrderSide) -> Option<Decimal> {
        match side {
            OrderSide::Buy => {
                let mut current = self.head.clone();
                let mut maybe_next = current.borrow().next[0].clone();

                while let Some(next) = maybe_next {
                    if !next.borrow().orders.is_empty() {
                        return Some(next.borrow().price);
                    }
                    current = next;
                    maybe_next = current.borrow().next[0].clone();
                }
            }
            OrderSide::Sell => {
                let mut current = self.head.clone();
                let mut maybe_next = current.borrow().next[0].clone();
                while let Some(next) = maybe_next {
                    if !next.borrow().orders.is_empty() {
                        return Some(next.borrow().price);
                    }
                    current = next;
                    maybe_next = current.borrow().next[0].clone();
                }
            }
        }
        None
    }

    pub fn get_orders_at_price(&self, price: Decimal) -> Option<&Vec<Order>> {
        self.price_map.get(&price)
    }

    pub fn get_depth(&self, depth: usize) -> Vec<OrderBookEntry> {
        let mut result = Vec::new();
        let mut current = self.head.clone();
        let mut count = 0;

        let mut maybe_next = current.borrow().next[0].clone();
        while let Some(next) = maybe_next {
            if count >= depth {
                break;
            }
            if !next.borrow().orders.is_empty() {
                let total_quantity = next.borrow().orders.iter().map(|o| o.quantity).sum();
                result.push(OrderBookEntry {
                    price: next.borrow().price,
                    quantity: total_quantity,
                    order_count: next.borrow().orders.len() as u64,
                });
                count += 1;
            }
            current = next;
            maybe_next = current.borrow().next[0].clone();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderStatus, OrderType};
    use chrono::Utc;

    fn create_test_order(price: Decimal) -> Order {
        Order {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            symbol: "BTC/USDT".to_string(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(price),
            quantity: Decimal::from(1),
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            iceberg_visible_quantity: None,
            stop_price: None,
            trailing_stop_price: None,
        }
    }

    #[test]
    fn test_add_and_remove_order() {
        let mut orderbook = SkipListOrderBook::new();
        let order = create_test_order(Decimal::from(100));
        let order_id = order.id;

        orderbook.add_order(order);
        assert_eq!(orderbook.size, 1);

        let removed = orderbook.remove_order(order_id, Decimal::from(100));
        assert!(removed.is_some());
        assert_eq!(orderbook.size, 0);
    }

    #[test]
    fn test_get_best_price() {
        let mut orderbook = SkipListOrderBook::new();
        let order1 = create_test_order(Decimal::from(100));
        let order2 = create_test_order(Decimal::from(200));

        orderbook.add_order(order1);
        orderbook.add_order(order2);

        let best_price = orderbook.get_best_price(OrderSide::Buy);
        assert_eq!(best_price, Some(Decimal::from(100)));
    }

    #[test]
    fn test_get_depth() {
        let mut orderbook = SkipListOrderBook::new();
        for i in 1..=5 {
            let order = create_test_order(Decimal::from(i * 100));
            orderbook.add_order(order);
        }

        let depth = orderbook.get_depth(3);
        assert_eq!(depth.len(), 3);
        assert_eq!(depth[0].price, Decimal::from(100));
        assert_eq!(depth[1].price, Decimal::from(200));
        assert_eq!(depth[2].price, Decimal::from(300));
    }
}
