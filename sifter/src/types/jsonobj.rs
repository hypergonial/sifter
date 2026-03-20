use serde_json::{Map, Number};

pub trait JsonObject {
    /// The underlying JSON Value type.
    type ValueType;

    /// Try to interpret this JSON value as an object, returning a reference to the underlying map if successful.
    fn as_object(&self) -> Option<&Map<String, Self::ValueType>>;

    /// Try to interpret this JSON value as an object, returning a mutable reference to the underlying map if successful.
    fn as_object_mut(&mut self) -> Option<&mut Map<String, Self::ValueType>>;

    /// Try to interpret this JSON value as an array, returning a reference to the underlying vector if successful.
    fn as_array(&self) -> Option<&Vec<Self::ValueType>>;

    /// Try to interpret this JSON value as an array, returning a mutable reference to the underlying vector if successful.
    fn as_array_mut(&mut self) -> Option<&mut Vec<Self::ValueType>>;

    /// Try to interpret this JSON value as a string, returning a reference to the underlying string if successful.
    fn as_str(&self) -> Option<&str>;

    /// Try to interpret this JSON value as a number, returning a reference to the underlying number if successful.
    fn as_number(&self) -> Option<&Number>;

    /// Try to interpret this JSON value as a boolean, returning the underlying boolean if successful.
    fn as_bool(&self) -> Option<bool>;

    /// Try to interpret this JSON value as an unsigned integer, returning the underlying u64 if successful.
    fn as_u64(&self) -> Option<u64>;

    /// Try to interpret this JSON value as a signed integer, returning the underlying i64 if successful.
    fn as_i64(&self) -> Option<i64>;

    /// Try to interpret this JSON value as a floating-point number, returning the underlying f64 if successful.
    fn as_f64(&self) -> Option<f64>;

    /// Try to interpret this JSON value as null, returning () if successful.
    fn as_null(&self) -> Option<()>;

    /// Return true if this JSON value is an object.
    fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// Return true if this JSON value is an array.
    fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Return true if this JSON value is a string.
    fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    /// Return true if this JSON value is a number.
    fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    /// Return true if this JSON value is a boolean.
    fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Return true if this JSON value is a number and fits into a u64.
    fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Return true if this JSON value is a number and fits into an i64.
    fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Return true if this JSON value is a number and fits into an f64.
    fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    /// Return true if this JSON value is null.
    fn is_null(&self) -> bool {
        self.as_null().is_some()
    }
}

impl JsonObject for serde_json::Value {
    type ValueType = Self;

    fn as_object(&self) -> Option<&Map<String, Self::ValueType>> {
        self.as_object()
    }

    fn as_object_mut(&mut self) -> Option<&mut Map<String, Self::ValueType>> {
        self.as_object_mut()
    }

    fn as_array(&self) -> Option<&Vec<Self::ValueType>> {
        self.as_array()
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self::ValueType>> {
        self.as_array_mut()
    }

    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }

    fn as_number(&self) -> Option<&Number> {
        self.as_number()
    }

    fn as_bool(&self) -> Option<bool> {
        self.as_bool()
    }

    fn as_u64(&self) -> Option<u64> {
        self.as_u64()
    }

    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }

    fn as_null(&self) -> Option<()> {
        self.as_null()
    }
}
