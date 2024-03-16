pub mod unix_time {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(date.timestamp())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;
        NaiveDateTime::from_timestamp_opt(timestamp, 0)
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid timestamp: {}", timestamp)))
    }
}

pub mod unix_time_option {
    use core::fmt;

    use chrono::NaiveDateTime;
    use serde::{
        de::{Error, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S>(date: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *date {
            Some(date) => serializer.serialize_i64(date.timestamp()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionSecondsTimestampVisitor)
    }

    struct OptionSecondsTimestampVisitor;

    impl<'de> Visitor<'de> for OptionSecondsTimestampVisitor {
        type Value = Option<NaiveDateTime>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a unix timestamp in seconds or none")
        }

        /// Deserialize a timestamp in microseconds since the epoch
        fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            d.deserialize_i64(OptionSecondsTimestampVisitor)
        }

        /// Deserialize a timestamp in microseconds since the epoch
        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        /// Deserialize a timestamp in microseconds since the epoch
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }
    }
}
