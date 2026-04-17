use ux::{u6, u12, u18, u36};

// One's complement arithmetic operations based on Duane Crap's implementation

pub fn sign_extend_12_to_18(u: u12) -> u18 {
    let val = u32::from(u);
    if val > 0o3777 {
        u18::new(val | 0o770000)
    } else {
        u18::new(val)
    }
}

pub fn is_zero(val: u18) -> bool {
    val == u18::new(0)
}

pub fn complement_18(x: u18) -> u18 {
    u18::new(u32::from(!x) & 0o777777)
}

pub fn complement_36(x: u36) -> u36 {
    u36::new(u64::from(!x) & 0o777777_777777)
}

pub fn add_18(a: u18, b: u18) -> u18 {
    let sum = u32::from(a) + u32::from(b);
    let mut result = sum & 0o777777;

    if sum > 0o777777 {
        result += 1;
        result &= 0o777777;
    } else if result == 0o777777 {
        result = 0;
    }

    u18::new(result)
}

pub fn add_36(a: u36, b: u36) -> u36 {
    let sum = u64::from(a) + u64::from(b) + 1;
    u36::new(sum & 0o777777_777777)
}

pub fn sub_18(a: u18, b: u18) -> u18 {
    add_18(a, complement_18(b))
}

pub fn sub_36(a: u36, b: u36) -> u36 {
    let b_twos_comp = (u64::from(b) ^ 0o777777_777777) + 1;
    let sum = u64::from(a) + b_twos_comp;
    u36::new(sum & 0o777777_777777)
}

pub fn ones_to_signed_18(val: u18) -> i32 {
    let v = u32::from(val);
    if v > 0o377777 {
        -((0o777777 - v) as i32)
    } else {
        v as i32
    }
}

pub fn mul_18(a: u18, b: u18) -> u36 {
    let a_val = u64::from(u32::from(a));
    let a_neg = a_val > 131071;
    let a_mag = if a_neg { (!a_val) & 0o777777 } else { a_val };

    let b_val = u64::from(u32::from(b));
    let b_neg = b_val > 131071;
    let b_mag = if b_neg { (!b_val) & 0o777777 } else { b_val };

    let mut product = a_mag * b_mag;

    if a_neg {
        product = (!product) & 0o777777_777777;
    }
    if b_neg {
        product = (!product) & 0o777777_777777;
    }

    u36::new(product)
}

pub fn div(dividend: u36, divisor: u18) -> (u18, u18) {
    // Hardware restoring division: 36-bit dividend / 18-bit divisor
    let div_val = u64::from(dividend);
    let div_neg = div_val > 0o377777_777777;
    let div_mag = if div_neg { 0o777777_777777 - div_val } else { div_val };

    let sor_val = u32::from(divisor) as u64;
    let sor_neg = sor_val > 0o377777;
    let sor_mag = if sor_neg { 0o777777 - sor_val } else { sor_val } as u32;

    if sor_mag == 0 {
        return (u18::new(0), u18::new(0));
    }

    // Upper 18 bits = initial partial remainder, lower 18 bits = shift register
    let mut r = (div_mag >> 18) as u32;
    let mut q = (div_mag & 0o777777) as u32;

    for _ in 0..18 {
        r = ((r << 1) | ((q >> 17) & 1)) & 0o777777;
        q = (q << 1) & 0o777777;

        if r >= sor_mag {
            r -= sor_mag;
            q |= 1;
        }
    }

    // Apply signs: quotient negative if signs differ, remainder takes dividend sign
    let quot_neg = div_neg ^ sor_neg;
    let rem_neg = div_neg;

    let q = if quot_neg && q != 0 { 0o777777 - q } else { q };
    let r = if rem_neg && r != 0 { 0o777777 - r } else { r };

    (u18::new(q), u18::new(r))
}

pub fn rshift_18(val: u18, shift: u6) -> u18 {
    let v = u32::from(val);
    let k = u32::from(shift);
    let result = v >> k;

    if v & 0o400000 != 0 {
        let mask = ((1u32 << k) - 1) << (18 - k);
        u18::new((result | mask) & 0o777777)
    } else {
        u18::new(result)
    }
}

pub fn rshift_36(val: u36, shift: u6) -> u36 {
    let v = u64::from(val);
    let k = u32::from(shift);
    let result = v >> k;

    if v & 0o400000_000000 != 0 {
        let mask = ((1u64 << k) - 1) << (36 - k);
        u36::new((result | mask) & 0o777777_777777)
    } else {
        u36::new(result)
    }
}
