pub mod book {
    extern crate bigdecimal;
    use bigdecimal::BigDecimal;
    use bigdecimal::ToPrimitive;
    use cached::proc_macro::cached;
    use std::ops::{Mul, Add, Sub, Div};
    use std::str::FromStr;
    use std::cmp::max;
    use bigdecimal::RoundingMode;
    use bigdecimal::FromPrimitive;
    use bigdecimal::Zero;
    use cached::SizedCache;


    #[cached(
        type = "SizedCache<String, i64>",
        create = "{ SizedCache::with_size(1000000) }",
        convert = r#"{ format!("{}", value) }"#
    )]
    pub fn value_to_scale(value: f64) -> i64 {
        let decimal_places = (-value.log10()).ceil() as i64;
        max(0, decimal_places)
    }

    #[cached(
        type = "SizedCache<String, BigDecimal>",
        create = "{ SizedCache::with_size(1000000) }",
        convert = r#"{ format!("{}{}{}", decimal, group_size, group_lower) }"#
    )]
    pub fn group(decimal: BigDecimal, group_size: f64, group_lower: bool) -> BigDecimal {
        let scale = value_to_scale(group_size);
        let group_decimal = BigDecimal::from_f64(group_size).unwrap().with_scale_round(4, RoundingMode::HalfUp);
        let decimal = decimal.with_scale_round(4, RoundingMode::HalfUp); // this is to round the decimal imperfections like 100.30000001, 100.299999 for 100.3
        let rounding_mode: RoundingMode = if group_lower { RoundingMode::Floor } else { RoundingMode::Ceiling };
        let div = (decimal.clone() / group_decimal.clone()).with_scale_round(0, RoundingMode::Floor);
        println!("{} / {} = {}", decimal, group_decimal, div);
        let calculated = (div * group_decimal.clone()).with_scale_round(scale, RoundingMode::Floor);
        if calculated == decimal {
            println!("{} == {}", calculated, decimal);
            return calculated;
        }
        println!("{} != {}", calculated, decimal);
        let rounded_value = (calculated + if group_lower { BigDecimal::zero() } else { group_decimal }).with_scale_round(scale, rounding_mode);
        return rounded_value;
    }

    #[test]
    fn test_half_precision_above() {
        let group_size = 0.1;
        let value = BigDecimal::from_str("100.300000852854").unwrap();
        let grouped_decimal_bid = group(value.clone(), group_size, true);
        let grouped_decimal_ask = group(value.clone(), group_size, false);
        assert_eq!(grouped_decimal_bid, BigDecimal::from_str("100.3").unwrap(), "Bid did not round down");
        assert_eq!(grouped_decimal_ask, BigDecimal::from_str("100.3").unwrap(), "Ask did not round up");
    }


    #[test]
    fn test_half_above() {
        let group_size = 0.1;
        let value = BigDecimal::from_str("100.3599999999999994315658113919198513031005859375").unwrap();
        let grouped_decimal_bid = group(value.clone(), group_size, true);
        let grouped_decimal_ask = group(value.clone(), group_size, false);
        assert_eq!(grouped_decimal_bid, BigDecimal::from_str("100.3").unwrap(), "Bid did not round down");
        assert_eq!(grouped_decimal_ask, BigDecimal::from_str("100.4").unwrap(), "Ask did not round up");
    }

    #[test]
    fn test_half_below_with_division() {
        let group_size = 0.05;
        let value = BigDecimal::from_str("100.3599999999999994315658113919198513031005859375").unwrap();
        let grouped_decimal_bid = group(value.clone(), group_size, true);
        let grouped_decimal_ask = group(value.clone(), group_size, false);
        assert_eq!(grouped_decimal_bid, BigDecimal::from_str("100.35").unwrap(), "Bid did not round down");
        assert_eq!(grouped_decimal_ask, BigDecimal::from_str("100.4").unwrap(), "Ask did not round up");
    }

    #[test]
    fn test_decimal_precision_lower() {
        let group_size = 0.1;
        let value = BigDecimal::from_str("100.29999990005678").unwrap();
        let grouped_decimal_bid = group(value.clone(), group_size, true);
        let grouped_decimal_ask = group(value.clone(), group_size, false);
        assert_eq!(grouped_decimal_bid, BigDecimal::from_str("100.3").unwrap(), "Bid remain the same");
        assert_eq!(grouped_decimal_ask, BigDecimal::from_str("100.3").unwrap(), "Ask remain the same");
    }

    #[test]
    fn test_half_below() {
        let group_size = 0.1;
        let value = BigDecimal::from_str("100.3100058").unwrap();
        let grouped_decimal_bid = group(value.clone(), group_size, true);
        let grouped_decimal_ask = group(value.clone(), group_size, false);
        assert_eq!(grouped_decimal_bid, BigDecimal::from_str("100.3").unwrap(), "Bid did not round down");
        assert_eq!(grouped_decimal_ask, BigDecimal::from_str("100.4").unwrap(), "Ask did not round up");
    }


    #[test]
    fn test_half_precision_below() {
        let group_size = 0.1;
        let value = BigDecimal::from_str("100.2999999852854").unwrap();
        let grouped_decimal_bid = group(value.clone(), group_size, true);
        let grouped_decimal_ask = group(value.clone(), group_size, false);
        assert_eq!(grouped_decimal_bid, BigDecimal::from_str("100.3").unwrap(), "Bid did not round down");
        assert_eq!(grouped_decimal_ask, BigDecimal::from_str("100.3").unwrap(), "Ask did not round up");
    }
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
                let q = group(BigDecimal::from_str(&case.0.to_string()).unwrap(), case.1, case.3);
                assert_eq!(q, BigDecimal::from_str(&case.2.to_string()).unwrap());
            }
        }
    }
