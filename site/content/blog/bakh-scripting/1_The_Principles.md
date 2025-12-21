# Bakh: The Principles
When creating this website, I thought it would be a good place to showcase my language designing skills with a Bash-like scripting language with features more like my style.
## The Feel of Bash
The main guiding principle that Bash tends to follow is that everything is a file: commands are just executable files in path; all parameters are expected to be file names; stdin/stdout/stderr are all treated as files. Most of these are text files, but some commands will treat the contents of a file as expecting a special format: non-text files. 

While I am taking inspiration from this approach, I do not feel like it fits modern programming practices very well. The main thing I am taking is that files and, in particular, strings are very important, and central to almost everything.
## Typed programming
The main limitation of treating everything as a file is the safety of typed programming: if everything is strongly typed, then we can be quite precise about how we want to handle data. Ints in particular are fundamental to a lot of algorithms but are almost completely missing from bash. This realisation is what lead me to begin considering typed parameters of commands. This on its own is quite nice, but clearly has issues: how do we generate values to use in those parameters? The obvious answer is literals or variables, but that is not very helpful because we so far have no way of allowing the variables to actually vary. Then we might think that functions should exist which return values rather than working in the stdin/stdout/stderr framework. But that doesn't feel quite right to me in this context.
## The Bipartite-I/O model
To solve the problem of adjoining types into a file-based language, I decided to redefine what commands and functions are and unify them into one construct which I will generally call a function. 

A function is a procedure which takes stdin alongside typed parameters and outputs stdout alongside a typed output. This is enough to handle errors since the stderr data can be handled through our typed output lying in an error case of the output type with relevant information (similar to how rust uses Result for all user-handled errors). That said, since this is currently a relatively small project Bakh may not feature a dedicated Result type or the ability to define custom types, so error handling will have to handled through other means.

In this framework, commands are simply functions with a unit return type (or possibly a Result<(),E> return type if I ever get around to introducing the relevant features for it).