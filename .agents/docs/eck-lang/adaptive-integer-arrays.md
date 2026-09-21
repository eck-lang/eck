# Adaptive integer array storage

This document records the current adaptive `int[]` contract and the future
storage and vectorized execution work that may build on it. It is not a
replacement for the language contract in
[`syntax/array-declaration.md`](syntax/array-declaration.md): sections marked
as future directions remain proposals until they are benchmarked and accepted.

## Static and dynamic element contracts

An unannotated mutable array has `ArrayElementContract::Dynamic`. Every concrete
`Value` is accepted directly, so integer values, strings, booleans, null, and
nested arrays may coexist without conversion. The compiler may retain a current
element profile for optimization, but that profile is not a contract.

Explicit array annotations create the static contracts described below.

`int8[]`, `int16[]`, `int32[]`, `int64[]`, and `int128[]` declare a fixed
representation. Every successfully stored element must actually carry that
representation, an out-of-range or inexact value is rejected, and the array
never widens itself to accommodate one. The element may be evaluated with a
wider temporary representation, but the contract applies to the final value
that crosses into storage.

`int[]` declares adaptive signed-integer semantics. Its widening progression is
`int8 -> int16 -> int32 -> int64 -> int128 -> bigint`; an element may retain
the first representation in that progression that can represent its final
value. Fixed-width and adaptive destinations therefore make different promises,
and the implementation keeps them separate:

```text
int8[] / int16[] / ...   fixed-width representation contract
int[]                    adaptive signed integer semantic type
```

The current implementation stores one complete runtime value per element. Each
element therefore retains its actual base and subtype, including a widened
representation, while neighboring elements retain theirs. This per-element
identity is a current semantic and storage boundary, not a name-based lookup
contract.

The current array payload is one contiguous live window with reusable front and
back spare capacity. It grows or recenters when an end operation needs space;
copy-on-write reuses the existing `Value` ownership model, and lexical slot
cleanup releases expired owners deterministically. These are implementation
details, not source-level type syntax.

## Future direction: chunked adaptive storage

A logical `int[]` would be divided into contiguous chunks, and each chunk would
carry its own physical representation:

```text
logical int[]

chunk 0   indices 0..1023      physical int8
chunk 1   indices 1024..2047   physical int32
chunk 2   indices 2048..3071   physical int8
```

A store that needs a wider representation widens only the chunk it lands in,
instead of rewriting a hundred-million-element array because one outlier needs
`int32`. Chunks stay contiguous, cache-friendly, and SIMD-friendly, and they map
onto column-oriented processing far more directly than arbitrary per-element
tagged integers do.

## Future direction: sparse exceptional values

Instead of widening a chunk, its base storage could stay narrow and only the
exceptionally wide elements could be held aside:

```text
base storage   int8[]
promoted exceptions
    index 42     -> int32
    index 9812   -> int64
```

A bitmap with a dense exception buffer, sorted sparse indices, or per-chunk
exception tables are all viable structures for that; a hash map is not the
default answer. The trade-offs are real and have to be measured:

* reads gain a branch or a look-up on the paths that can hit an exception;
* SIMD behaviour degrades, because a contiguous run is no longer uniformly wide;
* random access becomes more complicated than indexing a chunk;
* a loop cannot treat a full chunk as one homogeneous vector;
* memory use is excellent when promotions are extremely rare.

Sparse promotion could also live inside a chunked representation as the policy
of one chunk rather than as a global layout.

## Future SIMD and vectorized operations

Operations over adaptive storage should be designed around physical
representation groups rather than around one width for the whole array. Each
chunk would dispatch the kernel its own representation supports:

```text
chunk<int8>   -> int8 SIMD kernel
chunk<int32>  -> int32 SIMD kernel
chunk<int64>  -> int64 SIMD kernel
```

The language-level result stays a single logical array, while physical execution
may run different kernels for different chunks and combine their results. An
expression such as `c = a + b` could therefore become a sequence of chunk-local
kernel invocations that preserve ordinary ECK semantics.

The exact vectorized-array contract is not defined. Controlled benchmarks are
required before choosing chunking, sparse exceptions, or a kernel contract.
The physical layout must not force a semantic difference.

## Widening policy and materialization

Automatic widening is triggered by the final value being stored, not by a
temporary width used while its expression was evaluated. An expression that
promotes during evaluation and finishes with a value representable by a
narrower adaptive width may use that narrower width; a fixed-width destination
still requires its exact declared representation. Unit conversion for a fixed
integer destination must establish exact representability before division or
truncation.

Ordinary mutation should widen monotonically along
`int8 -> int16 -> int32 -> int64 -> int128 -> bigint`. Automatically narrowing
again after individual stores is not worth the repeated widen/compact
oscillation; a later explicit compaction or repacking phase may choose narrower
representations when that pays off.

An array that has grown fragmented may eventually be cheaper to materialize into
one common representation than to keep dispatching per chunk. Candidate signals
for that decision include the share of elements or chunks that need a wider
storage, the number of distinct physical widths, the expected upcoming
vectorized workload, per-chunk dispatch cost, memory overhead, cache behaviour,
and SIMD throughput. A number such as "materialize once 50% of the elements need
a wider width" is only a candidate for benchmarking; it is not a language rule,
and no threshold should be adopted without measurement.

## How this fits the current compiler and runtime boundary

The fixed-width fix introduced one destination contract at the point where a
value enters array storage. The compiler resolves an element destination and the
container applies exactly the work that destination requires:

```text
fixed representation   normalize to the declared representation, or reject
adaptive int            store the representation the expression produced
```

A destination the compiler proves already satisfies the declared representation
crosses into storage without a runtime check, so a provably safe element store
costs nothing. An element that may have promoted is evaluated first, then
normalized or rejected before the array is touched, which keeps a failed store
from modifying the stored element.

A future adaptive physical layout plugs into the same boundary rather than
duplicating the store pipeline: the destination for `int[]` would keep the
adaptive contract and gain the policy the storage implements, such as storing
directly, widening the affected chunk, or requesting materialization. The
current compiler resolves complete runtime element identities and dynamic
conversion/index plans into dense IR tables; the runtime selects those plans
without name lookup in hot paths. A genuinely open operation site uses the
stored runtime identity to query the Registry on a cache miss and retains the
prepared plan in a bounded inline cache. Chunking, sparse promotion, SIMD
kernels, and materialization thresholds remain deferred until controlled
benchmarks justify them.
