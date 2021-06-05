/// Moves head in $buffer $amount forward
macro_rules! advance {
    ($buffer:expr, $amount:expr) => {
        let (_, rest) = $buffer.split_at($amount);

        $buffer = rest
    };
}
