# Test Markdown with LaTeX

This is a test document to verify that the pretty command works correctly with markdown and LaTeX rendering.

## Basic Markdown Features

Here's some **bold text** and *italic text*.

- Bullet point 1
- Bullet point 2
- Bullet point 3

1. Numbered item
2. Another item
3. Third item

## Inline Math

The quadratic formula is $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$.

The Pythagorean theorem states that $a^2 + b^2 = c^2$.

## Display Math

Euler's identity:

$$e^{i\pi} + 1 = 0$$

The Gaussian integral:

$$\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}$$

## Code Block

```rust
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n-1) + fibonacci(n-2)
    }
}
```

## Conclusion

This document tests various markdown and LaTeX features.
