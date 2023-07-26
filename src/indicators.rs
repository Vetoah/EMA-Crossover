use std::collections::VecDeque;

pub fn ema(ema_vec: &VecDeque<f64>, closing_price_vec: &VecDeque<f64>, period: usize) -> f64 {
    const SMOOTHING: f64 = 2.0;

    if ema_vec.len() == 0 && closing_price_vec.len() >= period {
        let mut temp = VecDeque::new();
        for _n in 0..period {
            temp.push_front(closing_price_vec[_n]);
        }
        return average(&temp);
    } else if ema_vec.len() > 0 {
        return closing_price_vec[0] * (SMOOTHING / ((1 + period) as f64)) + ema_vec[0] * (1.0 - (SMOOTHING / ((1 + period) as f64)));
    }
    else {
        return -1.0;    
    }
}

pub fn rsi(closing_price_vec: &VecDeque<f64>, period: usize, prev_avg_gain: f64, prev_avg_loss: f64) -> (f64, f64, f64) {
    if closing_price_vec.len() == period + 1 {
        let mut gain = VecDeque::new();
        let mut loss = VecDeque::new();
        let mut current_price = closing_price_vec[1];

        for _n in 2..period {
           let mut price_diff = current_price - closing_price_vec[_n];
           if price_diff > 0.0 {
                gain.push_front(price_diff.abs());
                loss.push_front(0.0);
           } 
           else {
                loss.push_front(price_diff.abs());
                gain.push_front(0.0);
           }
           current_price = closing_price_vec[_n];
        } 
        let rs: f64 = average(&gain) / average(&loss);
        return (100.0 - (100.0 / ( 1.0 + rs )) ,average(&gain) , average(&loss)) ;
    }
    else if closing_price_vec.len() > period + 1 {
        let mut rs: f64 = 0.0;
        let mut price_diff = closing_price_vec[0] - closing_price_vec[1];
        let mut avg_gain = 0.0;
        let mut avg_loss = 0.0;

        if price_diff > 0.0 {
            avg_gain = ( (prev_avg_gain * (period as f64 - 1.0) + price_diff ) / period as f64);
            avg_loss = ((prev_avg_loss * (period as f64 - 1.0) ) / period as f64);
            rs = avg_gain / avg_loss;
        } 
        else {
            avg_gain = ( (prev_avg_gain * (period as f64 - 1.0)) / period as f64 );
            avg_loss = ( (prev_avg_loss * (period as f64 - 1.0) + price_diff.abs() ) / period as f64);
            rs =  avg_gain / avg_loss
        }
        return (100.0 - (100.0 / ( 1.0 + rs )), avg_gain, avg_loss);
    }
    
    else {
        return (-1.0, -1.0, -1.0)
    }
}

pub fn average(random_vec: &VecDeque<f64>) -> f64{
    let sum: f64 = random_vec.iter().sum();
    let count = random_vec.len() as f64;
    return sum / count;
    }


    