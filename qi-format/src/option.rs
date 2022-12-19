use crate::Value;

pub type Option<'v> = std::option::Option<Value<'v>>;

#[cfg(test)]
mod tests {
    use crate::{from_bytes, to_bytes};

    #[test]
    fn test_option_as_value_ref() {
        todo!()
    }

    #[test]
    fn test_option_as_value_mut() {
        todo!()
    }

    #[test]
    fn test_option_into_value() {
        todo!()
    }

    #[test]
    fn test_option_serde() {
        todo!()
    }

    #[test]
    fn test_std_option_to_bytes() {
        assert_eq!(
            to_bytes(&Some(42)).unwrap(),
            vec![
                1, // bool: true
                42, 0, 0, 0 // i32
            ]
        );
        assert_eq!(to_bytes(&std::option::Option::<i32>::None).unwrap(), [0]);
    }

    #[test]
    fn test_std_option_from_bytes() {
        assert_eq!(
            from_bytes::<std::option::Option<i8>>(&[
                1,  // bool: true
                42, // value
                43, 44 // trailing data (ignored)
            ])
            .unwrap(),
            Some(42)
        );
        assert_eq!(
            from_bytes::<std::option::Option<i8>>(&[
                0, // bool: false
                // no value
                1, 2, // trailing data (ignored)
            ])
            .unwrap(),
            None,
        );
    }
}
