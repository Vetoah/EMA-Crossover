#![allow(warnings, unused)]
mod indicators;
extern crate sha2;
use reqwest;
use std::{time::{SystemTime, UNIX_EPOCH}, io::Read, error::Error,  };
use std::time::{Duration, Instant};
use tokio::time;
use std::collections::VecDeque;
use chrono::{NaiveDateTime, DateTime, Utc};
use serde_json::{json, Value};
use hex::encode as hex_encode;
use base64;
use ring::{hmac, signature};

use indicators::{ema, rsi, average};

#[tokio::main]

async fn main() {
    let client = reqwest::Client::new();

    //Required for live trading --------------------------
    const URL: &'static str = "https://api.phemex.com/spot/wallets?currency=BTC";
    const SECRET: &'static str = "";
    const ACCESS_TOKEN: &'static str = "";

    let url_path: &str = "/spot/wallets";
    let query_str: &str = "currency=BTC";
    let expiry = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 60;
    let expiry_str = expiry.to_string();
    let api_secret = base64_url::decode(SECRET).unwrap();  
    let hmac_str = format!("{}{}{}", url_path, query_str, expiry_str);
    let signed_key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.as_ref());
    let ring_signature = hmac::sign(&signed_key, hmac_str.as_bytes());
    let ring_signature_str: String = format!("{:?}", ring_signature);
    let signature = ring_sha256_to_str(&ring_signature_str);

    //Historic data backtesting --------------------------------------------------------------------------
    const TAKERS_FEE: f64 = 0.00075;
    const LEVERAGE: f64 = 20.0;
    const PERCENTAGE_BAL: f64 = 0.1;
    const RESOLUTION: i32 = 300; //300 seconds or 5 min
    const COIN: &str = "BTC";
    const PRICE_SCALE: f64 = 10000.0;
    const INIT_BAL: f64 = 1000.0;
    
    let timeLength = RESOLUTION * 999; //number of k-lines should be less than 1000
    let mut timestamp = VecDeque::new();
    let mut closing_price = VecDeque::new();
    let mut opening_price = VecDeque::new();
    let mut ema_50 = VecDeque::new();
    let mut ema_100 = VecDeque::new();
    let mut ema_200 = VecDeque::new();
    let mut ema_diff_wins = VecDeque::new();
    let mut ema_diff_losses = VecDeque::new();

    let mut rsi_val = 0.0;
    let mut avg_gain = -1.0;
    let mut avg_loss = -1.0;
    let mut stop_loss = 0.0;
    let mut take_profit = 0.0; 
    let mut trade_option = 0;
    let mut in_trade = false;
    let mut balance = INIT_BAL;
    let mut entry_price = 0.0;
    let mut is_price_set = false;
    let mut long_count = 0.0;
    let mut short_count = 0.0;
    let mut wins = 0.0;
    let mut losses = 0.0;
    let mut taker = 0.0;
    let mut ema_diff = 0.0;

    let to = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i32;
    let from = to - timeLength;

    let url = format!(
        "https://api.phemex.com/exchange/public/md/kline?symbol={}USD&to={}&from={}&resolution={}",
        COIN,
        to,
        from,
        RESOLUTION
    );

    let order_book = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to get response")
        .text()
        .await
        .expect("failed to get payload");

    let mut object: Value = serde_json::from_str(&order_book).unwrap();
    
    for n in 0..998{
        //This loop will go through the JSON object and push the value of each period into these vectors.
        //The purpose of doing this is so we can calculate the EMAs and RSI as those are based on price action.
        timestamp.push_front(object["data"]["rows"][n][0].to_string().parse::<i32>().unwrap());
        closing_price.push_front(object["data"]["rows"][n][6].to_string().parse::<f64>().unwrap() / PRICE_SCALE);
        opening_price.push_front(object["data"]["rows"][n][3].to_string().parse::<f64>().unwrap() / PRICE_SCALE);

        let datetime_utc = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp[0] as i64, 0), Utc);

        let _ema_50 = ema(&ema_50, &closing_price, 50); 
        let _ema_100 = ema(&ema_100, &closing_price, 100); 
        let _ema_200 = ema(&ema_200, &closing_price, 200); 
        let (rsi_val, prev_avg_gain, prev_avg_loss) = rsi(&closing_price, 14, avg_gain, avg_loss);
        
        //These values are required because the previous values are needed to calculate the current values after the first N periods
        avg_gain = prev_avg_gain;
        avg_loss = prev_avg_loss;

        //These will output -1.0 if the vector length is less than the period selected, which is 50 in this case
        //The vectors are popped if they exceed the required length because we don't want it to go on forever
        if _ema_50 != -1.0 {
            ema_50.push_front(_ema_50);
            if ema_50.len() > 50 
            {
                ema_50.pop_back();
            }
        }
        if _ema_100 != -1.0 {
            ema_100.push_front(_ema_100);
            if ema_100.len() > 100 {
                ema_100.pop_back();
            }
        }
        if _ema_200 != -1.0 {
            ema_200.push_front(_ema_200);
            if ema_200.len() > 200 {        
                ema_200.pop_back();
            }
        }
        
        if !in_trade {
            let (_trade_option, stop_l, take_p, ema_diff_tot) = trade(rsi_val, &ema_50, &ema_100, &ema_200, &opening_price, &closing_price);
            stop_loss = stop_l;
            take_profit = take_p;
            ema_diff = ema_diff_tot;
           
            match _trade_option {
                1=> {                       
                        in_trade = true;
                        trade_option = 1; 
                    },
                2=> {                      
                        in_trade = true;
                        trade_option = 2; 
                    },
                3 => (),
                _=> (),
            }
        } else {
            //Prevents entry price value from being altered when it is in a trade
            if !is_price_set {
                entry_price = object["data"]["rows"][n][3].to_string().parse::<f64>().unwrap() / PRICE_SCALE;
                is_price_set = true;
            }

            //Prevents the trade from occuring if the entry price is outside of the spread
            //This range is direction dependent
            let long_spread = stop_loss..take_profit;
            let short_spread = take_profit..stop_loss;

            if trade_option == 1 && !long_spread.contains(&entry_price) {
                in_trade = false;
                is_price_set = false;     
            } else if trade_option == 2 && !short_spread.contains(&entry_price) {
                in_trade = false;
                is_price_set = false;   
            }

            if trade_option == 1 && in_trade {
                //This backtest method assumes the price hits the take profit before it hits the stop loss if overlapping occurs since there is no way of knowing.
                if object["data"]["rows"][n][4].to_string().parse::<f64>().unwrap() / PRICE_SCALE > take_profit{
                    let gain = (take_profit - entry_price) / take_profit * LEVERAGE * (PERCENTAGE_BAL * balance);
                    taker += LEVERAGE * (PERCENTAGE_BAL * balance) * TAKERS_FEE;
                    balance += gain;
                    wins += 1.0;
                    long_count += 1.0;
                    is_price_set = false;
                    in_trade = false;
                    trade_option = 0;
                    ema_diff_wins.push_front(ema_diff);
                    println!("Timestamp: {:.2} Entry Price {:.2} Take Profit: {:.2} Stop Loss: {:.2} PNL: +{:.2}", datetime_utc, entry_price, take_profit, stop_loss, gain);
                } else if object["data"]["rows"][n][5].to_string().parse::<f64>().unwrap() / PRICE_SCALE < stop_loss {
                    let loss = (take_profit - entry_price) / take_profit * LEVERAGE * (PERCENTAGE_BAL * balance);
                    taker += LEVERAGE * (PERCENTAGE_BAL * balance) * TAKERS_FEE;
                    balance -= loss;
                    losses += 1.0;
                    long_count += 1.0;
                    is_price_set = false;
                    in_trade = false;
                    trade_option = 0;
                    ema_diff_losses.push_front(ema_diff);
                    println!("Timestamp: {:.2} Entry Price {:.2} Take Profit: {:.2} Stop Loss: {:.2} PNL: -{:.2}", datetime_utc, entry_price, take_profit, stop_loss, loss);
                }
                
            } else if trade_option == 2 && in_trade {
                if object["data"]["rows"][n][4].to_string().parse::<f64>().unwrap() / PRICE_SCALE < take_profit {
                    let gain = -1.0 * (take_profit - entry_price) / take_profit * LEVERAGE * (PERCENTAGE_BAL * balance);
                    taker = LEVERAGE * (PERCENTAGE_BAL * balance) * TAKERS_FEE;
                    balance += gain;
                    wins += 1.0;
                    short_count += 1.0;
                    is_price_set = false;
                    in_trade = false;
                    trade_option = 0;
                    println!("Timestamp: {:.2} Entry Price {:.2} Take Profit: {:.2} Stop Loss: {:.2} PNL +{:.2}", datetime_utc, entry_price, take_profit, stop_loss, gain);
                } else if object["data"]["rows"][n][5].to_string().parse::<f64>().unwrap() / PRICE_SCALE > stop_loss {
                    let loss = -1.0 *(take_profit - entry_price) / take_profit * LEVERAGE * (PERCENTAGE_BAL * balance);
                    taker += LEVERAGE * (PERCENTAGE_BAL * balance) * TAKERS_FEE;
                    balance -= loss;
                    losses += 1.0;
                    short_count += 1.0;
                    is_price_set = false;
                    in_trade = false;
                    trade_option = 0;
                    println!("Timestamp: {:.2} Entry Price {:.2} Take Profit: {:.2} Stop Loss: {:.2} PNL: -{:.2}", datetime_utc, entry_price, take_profit, stop_loss, loss);
                }
            }
        } 
    }

    balance -= taker;

    println!("\nLongs: {:#?}", long_count);
    println!("Shorts: {:#?}\n", short_count);
    println!("Wins: {:#?}", wins);
    println!("Losses: {:#?}\n", losses);
    println!("Using {}% of balance per trade: {} {}x leverage", PERCENTAGE_BAL * 100.0, COIN, LEVERAGE);
    println!("Original Balance: {:.2}, Current Balance: {:.2}, Fees: {:.2}\n", INIT_BAL, balance, taker);
    println!("Average EMA diff of wins: {}, Average EMA diff of losses: {}", average(&ema_diff_wins), 
    average(&ema_diff_losses));
}

fn trade(rsi_val: f64, ema50: &VecDeque<f64>, ema100: &VecDeque<f64>, ema200: &VecDeque<f64>, opening_price: &VecDeque<f64>, closing_price: &VecDeque<f64>) -> (i32, f64, f64, f64) {
    let mut engulfing_bullish = false;
    let mut engulfing_bearish = false;
    let mut stop_loss = 0.0;
    let mut take_profit = 0.0; 
      
    if ema200.len() > 199 {  
        //previous candle is red && last closing price is greater than the current opening price && current closing price is greater than the last opening price
        if (closing_price[1] < opening_price[1] && closing_price[1] > opening_price[0] && closing_price[0] > opening_price[1]) {  
            engulfing_bullish = true;
        }
        
        if (opening_price[1] < closing_price[1] && closing_price[0] < opening_price[1] && opening_price[0] > closing_price[1]) {
            engulfing_bearish = true;
        }

        if rsi_val > 50.0 && rsi_val < 70.0 && ema50[0] > ema100[0] && ema100[0] > ema200[0] && engulfing_bullish {
            take_profit = closing_price[0] + 2.0 * (closing_price[0] - opening_price[0]);
            stop_loss = opening_price[0] - (closing_price[0] - opening_price[0]);
            let ema_diff_tot = (ema50[0] - ema100[0]) + (ema100[0] - ema200[0]);
            return (1, stop_loss, take_profit, ema_diff_tot)
        } else if rsi_val > 30.0 && rsi_val < 50.0 && ema50[0] < ema100[0] && ema100[0] < ema200[0] && engulfing_bearish {
            take_profit =  closing_price[0] - 2.0 *(opening_price[0] - closing_price[0]);
            stop_loss = opening_price[0] + (opening_price[0] - closing_price[0]);
            let ema_diff_tot = (ema200[0] - ema100[0]) + (ema100[0] - ema50[0]);
            return (2, stop_loss, take_profit,ema_diff_tot)
        }
    }
    return (3, 0.0, 0.0, 0.0)
}

fn ring_sha256_to_str(value: &str) -> &str {
    if value.len() >= 12 {
        &value[11..value.len() - 1]
    } else {
        ""
    }
}

