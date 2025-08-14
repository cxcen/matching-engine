use chrono::Utc;
use matching_engine::{engine::MatchingEngine, event_store::InMemoryEventStore, types::{OrderSide, OrderType}, PlaceOrderCommand};
use rust_decimal::Decimal;
use uuid::Uuid;

fn create_test_order_cmd(price: Decimal, quantity: Decimal, side: OrderSide) -> PlaceOrderCommand {
    PlaceOrderCommand {
        order_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        symbol: "BTC/USDT".to_string(),
        order_type: OrderType::Limit,
        side,
        price: Some(price),
        quantity,
        iceberg_visible_quantity: None,
        stop_price: None,
        trailing_stop_price: None,
        timestamp: Utc::now()
    }
}

#[tokio::test]
async fn test_basic_matching() {
    let event_store = Box::new(InMemoryEventStore::new());
    let engine = MatchingEngine::new(event_store);

    // Place buy order
    let buy_order = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(1),
        OrderSide::Buy,
    );
    let buy_events = engine.handle_place_order(buy_order).await.unwrap();
    assert_eq!(buy_events.len(), 1); // Only OrderPlaced event

    // Place sell order that should match
    let sell_order = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(1),
        OrderSide::Sell,
    );
    let sell_events = engine.handle_place_order(sell_order).await.unwrap();
    assert_eq!(sell_events.len(), 1); // OrderPlaced and OrderMatched events

    // Verify order book is empty
    let order_book = engine.get_order_book("BTC/USDT").unwrap();
    assert!(order_book.bids.is_empty());
    assert!(order_book.asks.is_empty());
}

#[tokio::test]
async fn test_partial_matching() {
    let event_store = Box::new(InMemoryEventStore::new());
    let engine = MatchingEngine::new(event_store);

    // Place buy order
    let buy_order = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(2),
        OrderSide::Buy,
    );
    let _ = engine.handle_place_order(buy_order).await.unwrap();

    // Place sell order that should partially match
    let sell_order = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(1),
        OrderSide::Sell,
    );
    let sell_events = engine.handle_place_order(sell_order).await.unwrap();
    assert_eq!(sell_events.len(), 2); // OrderPlaced and OrderMatched events

    // Verify remaining buy order
    let order_book = engine.get_order_book("BTC/USDT").unwrap();
    assert_eq!(order_book.bids.len(), 1);
    assert!(order_book.asks.is_empty());
}

#[tokio::test]
async fn test_price_priority() {
    let event_store = Box::new(InMemoryEventStore::new());
    let engine = MatchingEngine::new(event_store);

    // Place multiple buy orders at different prices
    let buy_order1 = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(1),
        OrderSide::Buy,
    );
    let buy_order2 = create_test_order_cmd(
        Decimal::from(101),
        Decimal::from(1),
        OrderSide::Buy,
    );
    let _ = engine.handle_place_order(buy_order1).await.unwrap();
    let _ = engine.handle_place_order(buy_order2).await.unwrap();

    // Place sell order that should match with the higher price
    let sell_order = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(1),
        OrderSide::Sell,
    );
    let sell_events = engine.handle_place_order(sell_order).await.unwrap();
    assert_eq!(sell_events.len(), 2); // OrderPlaced and OrderMatched events

    // Verify remaining buy order
    let order_book = engine.get_order_book("BTC/USDT").unwrap();
    assert_eq!(order_book.bids.len(), 1);
    assert_eq!(order_book.bids[0].price, Decimal::from(100));
    assert!(order_book.asks.is_empty());
}

#[tokio::test]
async fn test_market_order() {
    let event_store = Box::new(InMemoryEventStore::new());
    let engine = MatchingEngine::new(event_store);

    // Place limit sell order
    let sell_order = create_test_order_cmd(
        Decimal::from(100),
        Decimal::from(1),
        OrderSide::Sell,
    );
    let _ = engine.handle_place_order(sell_order).await.unwrap();

    // Place market buy order
    let mut market_buy = create_test_order_cmd(
        Decimal::from(0),
        Decimal::from(1),
        OrderSide::Buy,
    );
    market_buy.order_type = OrderType::Market;
    market_buy.price = None;
    let market_events = engine.handle_place_order(market_buy).await.unwrap();
    assert_eq!(market_events.len(), 2); // OrderPlaced and OrderMatched events

    // Verify order book is empty
    let order_book = engine.get_order_book("BTC/USDT").unwrap();
    assert!(order_book.bids.is_empty());
    assert!(order_book.asks.is_empty());
} 