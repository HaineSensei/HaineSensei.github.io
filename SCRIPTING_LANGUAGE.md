# KH Language Specification

A bash-like scripting language for terminal automation and filesystem exploration.

## File Extension

`.kh` files define functions and scripts.

## Execution Model

**Function call**: `function_name args...` - executes the function
**Script execution**: `script.kh` - runs global scope code

All `.kh` files in PATH directories are scanned for function definitions.

## Comments

```
# Single-line comment to end of line
$x: Int = 42  # Inline comment
```

## Types

**Primitives**: `String`, `Int`, `Bool`, `Path`, `File`, `Dir`, `()`

**Compound**:
- Tuples: `(T1, T2, ..., Tn)`, 1-tuple requires trailing comma: `(T,)`
- Lists: `List<T>`
- Options: `Option<T>` (used for flag parameters)

All types have `parse` and `to_string` methods.

## Tokenization

Input split on whitespace except within `"..."` quotes.

**Escape sequences** (inside quotes only): `\"`, `\n`, `\\`

**Token classification**:
- `-[a-zA-Z].*` → flag
- `-[0-9].*` → negative number
- Other → word/value

## Function Syntax

```
fn name (req: R) !(opt: O) *(var: V) -flag (fp: F) : ReturnType {
    # Function body
}
```

**Parameter modifiers**:
- `(x: T)` - required
- `!(x: T)` - optional (only at terminals)
- `*(xs: T)` - variadic (only at terminal, max one)

**Ordering rule**: `(required)* (optional)* (variadic)?`

Applies to both main parameters and per-flag parameters independently.

**Extra arguments**: Arguments beyond declared parameters are silently ignored.

**Examples**:
```
fn echo *(words: String) { ... }
fn help !(command: String) -v { ... }
fn process (input: String) !(mode: String) -output (file: Path) *(opts: String) { ... }

# Calling:
help ls extra_arg        # extra_arg ignored
```

## Variables

Variables marked with `$` prefix.

**Declaration** (type required):
```
$x: Int = 42
$name: String = "value"
```

**Reassignment** (type inferred):
```
$x = mul 2 $x
```

**Reference** (required for variable values):
```
echo $x
$y: Int = add $x 10
```

**Semantics**: `$var` interpolates via `var.to_string()`, then parses as target type.
**Optimization**: When types match, direct reference (no string conversion).

## Contexts

**Command context** contains:
- Command calls
- Control flow
- `return` statements
- Variable definitions

**Expression context**: Single expression producing typed value.

**Global scope** (`.kh` top-level): Sequence of command contexts.

**Function body** (with return type): Sequence of command contexts + final expression.

## Control Flow

**if**:
```
if Expression<Bool> {
    # Commands
}
```

**if/else**:
```
if Expression<Bool> {
    # Commands
} else {
    # Commands
}
```

**while**:
```
while Expression<Bool> {
    # Commands
}
```

**for** (integer range, exclusive end):
```
for $i = Expression<Int> until Expression<Int> {
    # Commands
}
```

**break**: Exit loop early (valid in `while`/`for`).

**No**: `continue`, `else if` (nest `if` in `else` instead).

## Expressions

Valid in `Expression<T>` contexts:
- Literals parsed as T (e.g., `42`, `"hello"`, `true`, `false`)
- `$variable` (interpolated and parsed as T)
- Function calls returning T

**Function application** is left-associative (Haskell-style). Use parentheses to group sub-expressions.

**No operators** - all operations are function calls:
```
$sum: Int = add 5 10
$product: Int = mul $sum 3
$cond: Bool = and (eq $x 42) (less $y 10)
```

## Flag Parameters

Flags scoped to `if -flag` blocks:

```
fn cmd (x: String) -verbose (level: Int) {
    echo $x

    if -verbose {
        echo "Verbosity level: "
        echo $level  # Only accessible here
    }
}
```

## Return Values

**Explicit return** (from command context):
```
return Expression<T>
```

**Implicit return** (final expression in function body):
```
fn add (a: Int) (b: Int) : Int {
    echo "Adding..."
    add $a $b  # Final expression, returns result
}
```

**Functions without return type** (`: ()` or omitted): No final expression required.

## Pass-by-Reference and Mutability

All parameters passed by reference for efficiency.

**Mutable parameters** (declared in signature only):
```
fn push (val: T) (mut vals: List<T>) {
    # Can mutate vals
}
```

**Call site** (no `mut` keyword):
```
$my_list: List<Int> = empty_list
push 42 $my_list
```

**Compiler analysis**: Variables assigned from other variables are cloned if used mutably, referenced otherwise.

## stdout vs Return Values

Functions have both stdout (string) and return values (typed).

**Command context** (terminal, pipes):
- Uses stdout
- Return values computed but ignored
- `return` still terminates function

**Expression context** (assignments, expressions):
- Uses return value
- stdout generated but ignored

**stdout accumulation**: Each inner command appends to function's stdout.

```
fn example (x: Int) : Int {
    echo "Starting"    # stdout += "Starting\n"
    echo $x            # stdout += "42\n"
    mul $x 2           # Returns 84, stdout contains "Starting\n42\n"
}

# Command context:
example 42 | other_cmd  # Pipes "Starting\n42\n"

# Expression context:
$result: Int = example 42  # Gets 84, stdout ignored
```

## Piping

Standard bash-style piping in command context:
```
cmd1 | cmd2 | cmd3
```

Pipes connect stdout → stdin as strings.

## Semicolons

Optional statement separators, equivalent to line breaks (except in comments):
```
$x: Int = 42; $y: Int = 43; echo $x
```

## Standard Library (Planned)

**Arithmetic**: `add`, `sub`, `mul`, `div`, `mod`
**Comparison**: `eq`, `less`, `greater`, `less_eq`, `greater_eq`
**Logic**: `and`, `or`, `not`
**Lists**: `empty_list`, `push`, `pop`, `length`, `get`, `set`
**IO**: `echo`, `readln` (read line from stdin), `cat`
**Filesystem**: `ls`, `cd`, `pwd`, `mkdir`, `rm`

## Implementation Notes

- `.kh` files recompiled on edit
- Compilation errors shown as `SyntaxError: ...` at runtime
- Function name collisions trigger warning at compile time
- PATH directory scanned for all available functions
- Reference semantics like Python (primitives act like copies, compounds are references)
