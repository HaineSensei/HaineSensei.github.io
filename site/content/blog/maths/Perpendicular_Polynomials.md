# Perpendicular Polynomials

In this post, we will look at inner products on polynomials. For simplicity, I will restrict to real inner products, where the notion of perpendicular polynomials makes the most sense. Much of this discussion can be generalised to other fields, and so I will leave the details fairly open to interpretation.

I will denote $\mathbb{R}[x]$ by $P$ and the subspace of polynomials of degree at most $n$ by $P_n$ where we use the convention that $\deg({0}) \le 0$. I will also call the basis $\{1, x, x^2, \dots \}$ of $P$ the monomial basis.

We will look at some particular inner products. They are quite different and thus have different points of interest, so we will not follow particularly comparable discussion paths. We will, however, mention orthogonal bases in each case since this provides a good description of their structure.

## Coefficient Inner Product

To someone coming from a background in finite dimensional vector spaces, this is probably the most obvious inner product on $P$. It is defined simply as:

$$\langle p, q\rangle_\text{coef} := \sum_{n = 0}^\infty a_n b_n$$

where $a_i$ and $b_i$ are the coefficients of $x^i$ in $p$ and $q$ respectively. 

When we restrict this, we get an obvious isomorphism between $P_n$ and $\mathbb{R}^{n+1}$ with the standard inner product for any $n$. Since these subspaces cover $P_n$ it follows fairly simply that $\langle \cdot, \cdot \rangle_\text{coef}$ is an inner product on $P$.

I promised mentioning an orthogonal basis, so I will: the monomial basis. This may seem obvious, and that is because we have essentially defined this inner product based on the assumption that this basis is an orthonormal basis.

## Derivative Inner Products

Unlike the coefficient inner product which feels like it is defined by the algebraic representation of the polynomials, the derivative inner products are defined based on intrinsic data to the polynomials as functions—values and derivatives:

$$\langle p, q\rangle_{x_0} := \sum_{n = 0}^\infty p^{(n)}(x_0) q^{(n)}(x_0)$$

If we look at the Taylor expansions about the point $x_0$

$$p(x) = \sum_{n = 0}^{\deg(p)} \frac{p^{(n)}(x_0)}{n!} (x - x_0)^n$$

and similar for $q$, we see that this is actually very similar to the coefficient inner product. Indeed, the set $\{1, x - x_0, (x - x_0)^2, \dots \}$ forms an orthogonal basis. We can normalise this to get $\{1, x - x_0, \frac{(x - x_0)^2}{2!}, \frac{(x - x_0)^3}{3!}, \dots \}$ which is an orthonormal basis.

In the case that $x_0 = 0$, the monomial basis is the above mentioned orthogonal basis which one can normalise with factors of $1/n!$. As such, the derivative inner product captures all the same data as the coefficient inner product up to the normalising scale factors through analytic methods instead of algebraic methods.

## $L^2$ Inner Products

These are inner products that arise from attempting to adapt inner products to function spaces. In general, we can construct an inner product

$$\langle p, q \rangle_S := \int_S p q \, d\mu$$

whenever $S$ is a measurable set with non-zero measure. But for convenience we will restrict this to the case that $S$ is an interval. And for our computations, even restrict further to $S = [-1, 1]$, so that we obtain

$$\langle p, q\rangle_{-1,1} := \int_{-1}^1 p(x) q(x) \, dx.$$

To begin investigating this, we will see how it acts on the monomial basis:

$$\begin{aligned}
\langle x^m, x^n \rangle_{-1,1} &= \int_{-1}^1 x^{m+n}\, dx \\
&= \left[ \frac{x^{m+n+1}}{m+n+1} \right]_{-1}^1 \\
&=
\begin{cases}
\frac{2}{m+n+1} & \text{if } m + n \text{ is even,} \\
0 & \text{otherwise.}
\end{cases}
\end{aligned}$$

This shows that $x^m$ and $x^n$ are orthogonal if and only if $n$ and $m$ have opposite odd/even parity, so the monomial basis is not already an orthogonal basis.

We will use the Gram-Schmidt orthogonalisation process to begin producing an orthogonal basis from the monomial basis:

- $1$

- $x$ (is already orthogonal to $1$)

- $x^2$: this is not orthogonal to $1$, so we remove the component orthogonal to $1$:

  $$x^2 - \frac{\langle x^2, 1 \rangle_{-1, 1}}{\langle 1, 1 \rangle_{-1,1}} \cdot 1 = x^2 - \frac{1}{3}$$

- $x^3$: orthogonal to both $1$ and $x^2$ so can ignore components including them, but is not orthogonal to $x$:

  $$x^3 - \frac{\langle x^3, x\rangle_{-1, 1}}{\langle x, x \rangle_{-1,1}} \cdot x = x^3 - \frac{3}{5}x$$

- $x^4$: orthogonal to both $x$ and $x^3$ but not to $1$ and $x^2$:

  $$\begin{aligned}
  & x^4 - \frac{\langle x^4, 1 \rangle_{-1,1}}{\langle 1, 1 \rangle_{-1,1}} \cdot 1 \\
  & \quad - \frac{\langle x^4, x^2 - \frac{1}{3} \rangle_{-1,1}}{\langle x^2 - \frac{1}{3}, x^2 - \frac{1}{3} \rangle_{-1,1}} \cdot \left( x^2 - \frac{1}{3} \right) \\
  &= x^4 - \frac{6x^2}{7} + \frac{3}{35}
  \end{aligned}$$

And so on...

This is also the inner product over which the Legendre polynomials form an orthogonal basis. The same process produces them, but, where I accepted each polynomial as it was, the Legendre polynomials normalise so that their value at $1$ is $1$. Indeed, the polynomials we produced are themselves unnormalised Legendre polynomials: 

The component we are subtracting from our polynomial $x^n$ is its orthogonal projection relative to this inner product. Structurally, this means that the image of $\text{Id} - \pi_{n-1}$ (where $\pi_{n-1}$ is this projection) is forced to be in the orthogonal component of $P_n$ to $P_{n-1}$ which is a $1$ dimensional subspace. Thus, all non-zero elements of its image are "parallel" ie. scalar multiples of each other.
