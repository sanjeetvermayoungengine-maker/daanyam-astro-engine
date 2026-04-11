# Mean Nodes

Mean lunar nodes are currently unsupported in the DE440 backend.

## Rationale

- The current node implementation is explicitly the true-node policy derived from the Moon's instantaneous orbital plane against the mean ecliptic of date.
- A distinct mean-node implementation would require its own documented derivation policy and dedicated regression fixtures before shipping.
- The project does not permit silent precision fallbacks or implied equivalence between true and mean nodes.

## Current behavior

- `EngineConfig.node_mode = true` is supported.
- `EngineConfig.node_mode = mean` returns a typed `UnsupportedOperation` error from the DE440 backend.

Mean nodes will remain unsupported until source-attributed or manual-reference fixtures are added first and validated end-to-end.
