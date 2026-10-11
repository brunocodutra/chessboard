use anyhow::Error as Failure;
use brotli::Decompressor;
use std::io::{BufWriter, Read, Write};
use std::{env, fs::File, path::Path};

include!("lib/nnue/arch.rs");

/// Reorders one accumulator row by [`PERMUTATION`].
fn permute<T: Copy>(row: &mut [T; L1]) {
    let mut dst = *row;

    for (i, &p) in PERMUTATION.iter().enumerate() {
        dst[i] = row[p];
        dst[i + L1 / 2] = row[p + L1 / 2];
    }

    *row = dst;
}

fn arrange_in_blocks<T: Copy, const I: usize, const O: usize, const B: usize, const M: usize>(
    input: &[[T; I]; O],
    output: &mut [[T; B]; M],
) {
    const { assert!(I.is_multiple_of(B)) };
    const { assert!(M == I * O / B) };

    let (chunks, remainder) = output.as_chunks_mut::<O>();
    assert!(remainder.is_empty());

    for (blk, dst) in chunks.iter_mut().enumerate() {
        for (i, row) in input.iter().enumerate() {
            dst[i].copy_from_slice(&row[blk * B..blk * B + B]);
        }
    }
}

fn interleave<T: Copy, const M: usize>(input: &[T; M], output: &mut [T; M], n: usize) {
    let (lo, hi) = input.split_at(M / 2);
    for (i, (a, b)) in Iterator::zip(lo.chunks_exact(n), hi.chunks_exact(n)).enumerate() {
        output[2 * i * n..2 * i * n + n].copy_from_slice(a);
        output[2 * i * n + n..2 * (i + 1) * n].copy_from_slice(b);
    }
}

fn main() -> Result<(), Failure> {
    let nnue = "lib/nnue/nnue.bin.br";
    println!("cargo:rerun-if-changed={nnue}");

    let out_dir = env::var("OUT_DIR")?;
    let dst = Path::new(&out_dir).join("nnue.bin");

    let mut input = Decompressor::new(File::open(nnue)?, 4096);
    let mut output = BufWriter::new(File::create(&dst)?);

    let mut row = [0i8; L1];
    for _ in 0..FEATURES {
        input.read_exact(bytemuck::bytes_of_mut(&mut row))?;
        permute(&mut row);
        output.write_all(bytemuck::bytes_of(&row))?;
    }

    let mut bias = [0i16; L1];
    input.read_exact(bytemuck::bytes_of_mut(&mut bias))?;
    permute(&mut bias);
    output.write_all(bytemuck::bytes_of(&bias))?;

    let mut specialists: [Vec<u8>; SPECIALISTS] = std::array::from_fn(|_| Vec::new());

    for specialist in &mut specialists {
        let mut weight = [[0i8; L1]; L2];
        input.read_exact(bytemuck::bytes_of_mut(&mut weight))?;
        weight.iter_mut().for_each(permute);
        let mut blocks = [[0i8; 4]; L1 * L2 / 4];
        arrange_in_blocks(&weight, &mut blocks);
        let mut dst = [[0i8; 4]; L1 * L2 / 4];
        interleave(&blocks, &mut dst, L2 * 2);
        specialist.extend_from_slice(bytemuck::bytes_of(&dst));
    }

    for specialist in &mut specialists {
        let mut bias = [0f32; L2];
        input.read_exact(bytemuck::bytes_of_mut(&mut bias))?;
        specialist.extend_from_slice(bytemuck::bytes_of(&bias));
    }

    for specialist in &mut specialists {
        let mut weight = [[0f32; L2]; (L3 - L2) * 2];
        input.read_exact(bytemuck::bytes_of_mut(&mut weight))?;
        let mut dst = [[0f32; 1]; L2 * (L3 - L2) * 2];
        arrange_in_blocks(&weight, &mut dst);
        specialist.extend_from_slice(bytemuck::bytes_of(&dst));
    }

    for specialist in &mut specialists {
        let mut bias = [0f32; (L3 - L2) * 2];
        input.read_exact(bytemuck::bytes_of_mut(&mut bias))?;
        specialist.extend_from_slice(bytemuck::bytes_of(&bias));
    }

    for specialist in &mut specialists {
        let mut weight = [0f32; L3];
        input.read_exact(bytemuck::bytes_of_mut(&mut weight))?;
        specialist.extend_from_slice(bytemuck::bytes_of(&weight));
    }

    for specialist in &mut specialists {
        let mut bias = 0f32;
        input.read_exact(bytemuck::bytes_of_mut(&mut bias))?;
        specialist.extend_from_slice(bytemuck::bytes_of(&[bias / L3 as f32; L3]));
    }

    for specialist in &specialists {
        output.write_all(specialist)?;
    }

    output.flush()?;

    Ok(())
}
