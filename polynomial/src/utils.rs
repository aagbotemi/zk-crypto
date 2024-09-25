use std::f64::consts::PI;

use crate::multilinear::coefficient_form::MultiLinearMonomial;
use ark_ff::{BigInteger, PrimeField, Zero};
use num_bigint::BigUint;
use num_complex::Complex;
use num_traits::ToPrimitive;

pub fn pick_pairs_with_index<F: PrimeField>(
    terms: &Vec<MultiLinearMonomial<F>>,
) -> Vec<(usize, usize)> {
    let n = terms.len();
    let mut pairs = Vec::with_capacity(n / 2);

    for i in 0..(n / 2) {
        let j = i + n / 2;
        pairs.push((i, j));
    }

    pairs
}

pub fn pick_pairs_with_random_index(
    num_of_evaluations: usize,
    variable_index: usize,
) -> Vec<(usize, usize)> {
    assert!(num_of_evaluations % 2 == 0, "n must be even");
    assert!(
        variable_index < num_of_evaluations / 2,
        "variable_index must be less than n/2"
    );

    let mut result = Vec::new();
    let iters = 1 << variable_index;

    for _ in 0..iters {
        let mut round: Vec<(usize, usize)> = Vec::new();

        for y_1 in 0..((num_of_evaluations / iters) / 2) {
            round.push((
                y_1 + result.len() * 2,
                ((num_of_evaluations / iters) / 2) + y_1 + result.len() * 2,
            ));
        }

        result.extend(round);
    }

    result
}

pub fn pick_pairs_with_random_n_index(
    num_of_evaluations: usize,
    variable_indices: &[usize],
) -> Vec<(usize, usize)> {
    assert!(num_of_evaluations % 2 == 0, "n must be even");
    assert!(
        variable_indices.len() < num_of_evaluations / 2,
        "variable_index must be less than n/2"
    );

    let mut pairs = Vec::new();

    for i in 0..num_of_evaluations {
        for &variable_index in variable_indices {
            let pair = (i, i ^ (1 << variable_index));
            if !pairs.contains(&pair) && !pairs.contains(&(pair.1, pair.0)) {
                pairs.push(pair);
            }
        }
    }
    pairs
}

pub fn lagrange_basis<F: PrimeField>(points: &[(F, F)], i: usize) -> Vec<F> {
    let mut l_i = vec![F::one()];

    for (j, &(x_j, _)) in points.iter().enumerate() {
        if i != j {
            let mut new_l_i = vec![F::zero(); l_i.len() + 1];
            for (k, &coeff) in l_i.iter().enumerate() {
                new_l_i[k] -= coeff * x_j;
                new_l_i[k + 1] += coeff;
            }
            l_i = new_l_i;
        }
    }

    let denom = points
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != i)
        .fold(F::one(), |acc, (_, &(x_j, _))| acc * (points[i].0 - x_j));
    l_i.into_iter()
        .map(|coeff| coeff * denom.inverse().unwrap())
        .collect()
}

pub fn boolean_hypercube<F: PrimeField>(n: usize) -> Vec<Vec<F>> {
    let mut hypercube = Vec::with_capacity(1 << n);

    for i in 0..(1 << n) {
        let mut vertex = Vec::with_capacity(n);
        for j in (0..n).rev() {
            if (i & (1 << j)) != 0 {
                vertex.push(F::one());
            } else {
                vertex.push(F::zero());
            }
        }
        hypercube.push(vertex);
    }

    hypercube
}

pub fn fft<F: PrimeField>(coefficients: &Vec<F>) -> Vec<Complex<f64>> {
    let length_of_coefficients = coefficients.len();

    if length_of_coefficients == 1 {
        return vec![Complex::new(
            convert_prime_field_to_f64(coefficients[0]),
            0.0,
        )];
    }

    // nth root of unity => Z^n = 1
    // 2π/n => e^iθ = cos(θ) + i.sin(θ)
    // ω = e^(2πi/n)
    let ω = nth_root_of_unity(length_of_coefficients, false);

    // Pe = [P0,P2,...,Pn-2]
    let poly_even = get_even_indexed_coefficients(&coefficients);
    // Po = [P1,P3,...,Pn-1]
    let poly_odd = get_odd_indexed_coefficients(&coefficients);

    // y_e = fft(Pe)
    let y_e = fft(&poly_even);
    // y_o = fft(Po)
    let y_o = fft(&poly_odd);

    // y = [0] * n
    let mut y = vec![Complex::zero(); length_of_coefficients];
    let half_len = length_of_coefficients / 2;

    for j in 0..half_len {
        let ω_pow_j = ω.powf(j as f64);
        y[j] = y_e[j] + (ω_pow_j * y_o[j]);
        y[j + half_len] = y_e[j] - (ω_pow_j * y_o[j]);
    }

    y
}

pub fn ifft(coefficients: &Vec<Complex<f64>>) -> Vec<Complex<f64>> {
    let length_of_coefficients = coefficients.len();
    if length_of_coefficients == 1 {
        return vec![coefficients[0]];
    }

    let ω = nth_root_of_unity(length_of_coefficients, true);

    let poly_even = get_even_indexed_coefficients(&coefficients);
    let poly_odd = get_odd_indexed_coefficients(&coefficients);

    let y_e = ifft(&poly_even);
    let y_o = ifft(&poly_odd);

    let mut y = vec![Complex::zero(); length_of_coefficients];
    let half_len = length_of_coefficients / 2;

    for j in 0..half_len {
        let ω_pow_j = ω.powf(j as f64);
        y[j] = y_e[j] + (ω_pow_j * y_o[j]);
        y[j + half_len] = y_e[j] - (ω_pow_j * y_o[j]);
    }

    y
}

fn nth_root_of_unity(n: usize, inverse: bool) -> Complex<f64> {
    let degree = if inverse {
        // -2π/n
        (-2.0 * PI) / (n as f64)
    } else {
        // 2π/n
        (2.0 * PI) / (n as f64)
    };

    // e^iθ = cos(θ) + i.sin(θ)
    Complex::new(degree.cos(), degree.sin())
}

pub fn get_even_indexed_coefficients<T: Clone>(input: &[T]) -> Vec<T> {
    input[..].iter().step_by(2).cloned().collect()
}

pub fn get_odd_indexed_coefficients<T: Clone>(input: &[T]) -> Vec<T> {
    input[1..].iter().step_by(2).cloned().collect()
}

pub fn convert_prime_field_to_f64<F: PrimeField>(input: F) -> f64 {
    let bigint = input.into_bigint();
    let biguint = BigUint::from_bytes_le(&bigint.to_bytes_le());
    biguint.to_f64().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Fq;

    #[test]
    fn test_boolean_hypercube() {
        let one: Vec<Vec<Fq>> = boolean_hypercube(1);
        let two: Vec<Vec<Fq>> = boolean_hypercube(2);
        let three: Vec<Vec<Fq>> = boolean_hypercube(3);

        let expected_one = vec![vec![Fq::from(0)], vec![Fq::from(1)]];
        let expected_two = vec![
            vec![Fq::from(0), Fq::from(0)],
            vec![Fq::from(0), Fq::from(1)],
            vec![Fq::from(1), Fq::from(0)],
            vec![Fq::from(1), Fq::from(1)],
        ];
        let expected_three = vec![
            vec![Fq::from(0), Fq::from(0), Fq::from(0)],
            vec![Fq::from(0), Fq::from(0), Fq::from(1)],
            vec![Fq::from(0), Fq::from(1), Fq::from(0)],
            vec![Fq::from(0), Fq::from(1), Fq::from(1)],
            vec![Fq::from(1), Fq::from(0), Fq::from(0)],
            vec![Fq::from(1), Fq::from(0), Fq::from(1)],
            vec![Fq::from(1), Fq::from(1), Fq::from(0)],
            vec![Fq::from(1), Fq::from(1), Fq::from(1)],
        ];

        assert_eq!(one, expected_one);
        assert_eq!(two, expected_two);
        assert_eq!(three, expected_three);
    }
}
