#[cfg(test)]
mod tests {
    use super::rewrite_slice_bound_aliases;

    #[test]
    fn rewrite_slice_bound_aliases_keeps_branch_chain_with_empty_else_blocks() {
        let mut input = [
            "Sym_83 <- function(dir, size) ",
            "{",
            "  start <- rr_idx_cube_vec_i(f, x, 1.0, size)",
            "  end <- rr_idx_cube_vec_i(f, x, size, size)",
            "  if (licm_28) {",
            "    neighbors[start:end] <- Sym_60(f, x, ys, size)",
            "  } else {",
            "  }",
            "  if (licm_35) {",
            "    neighbors[start:end] <- Sym_64(f, x, ys, size)",
            "  } else {",
            "  }",
            "  if (licm_47) {",
            "    neighbors[start:end] <- Sym_66(f, x, ys, size)",
            "  } else {",
            "  }",
            "  if (licm_59) {",
            "    neighbors[start:end] <- Sym_72(f, x, ys, size)",
            "  } else {",
            "  }",
            "  return(neighbors)",
            "}",
            "",
        ]
        .join("\n");
        rewrite_slice_bound_aliases(&mut input);
        assert!(!input.contains("start <- rr_idx_cube_vec_i"), "{input}");
        assert!(!input.contains("end <- rr_idx_cube_vec_i"), "{input}");
        for needle in [
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)",
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)",
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, ys, size)",
            "neighbors[rr_idx_cube_vec_i(f, x, 1.0, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, ys, size)",
        ] {
            assert!(input.contains(needle), "{input}");
        }
    }
}
