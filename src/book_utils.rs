pub mod book {
    extern crate bigdecimal;
    use bigdecimal::BigDecimal;
    use bigdecimal::ToPrimitive;
    use std::ops::{Mul, Add, Sub, Div};

    pub fn group_decimal(price:f64, group_size:f64, group_lower:bool) -> BigDecimal{
        let price_decimal = BigDecimal::from(price);
        let group_decimal = BigDecimal::from(group_size);
        group_bigdecimal(price_decimal, group_decimal, group_lower)
    }

    pub fn group_bigdecimal(price_decimal:f64, group_decimal:f64, group_lower:bool) -> BigDecimal{
        let quotient = price_decimal.div(group_decimal.clone()).to_i64().unwrap();
        let quotient_decimal = if group_lower { quotient } else { quotient + 1 };
        BigDecimal::from(quotient_decimal).mul(group_decimal)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_grouping() {
            let cases = vec![
                //value, grouping, result, 
                (4.324210 , 0.5, 4.00, true),
                (4.324210 , 0.05, 4.3, true),
                (4.324210 , 0.005, 4.32, true),
                (4.624210 , 0.5, 4.5, true),
                (4.624210 , 5.0, 0.0, true),
                (4.324210 , 0.5, 4.50, false),
                (4.324210 , 0.05, 4.35, false),
                (4.324210 , 0.005, 4.325, false),
                (4.624210 , 0.5, 5.0, false),
                (4.624210 , 5.0, 5.0, false),
            ];

            for case in &cases {
                let q = group_decimal(case.0, case.1, case.3);
                assert_eq!(q, BigDecimal::from(case.2));
            }
        }
    }
}