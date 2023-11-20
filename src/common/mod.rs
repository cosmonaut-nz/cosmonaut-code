//! A set of common

#[macro_export]
macro_rules! impl_builder_methods {
    // For non-optional fields
    ($builder:ident, $($field:ident: $field_type:ty),*) => {
        impl $builder {
            $(
                pub fn $field(mut self, $field: $field_type) -> Self {
                    self.$field = $field;
                    self
                }
            )*
        }
    };
    // For optional fields
    (opt, $builder:ident, $($field:ident: $field_type:ty),*) => {
        impl $builder {
            $(
                pub fn $field(mut self, $field: $field_type) -> Self {
                    self.$field = Some($field);
                    self
                }
            )*
        }
    };
}
