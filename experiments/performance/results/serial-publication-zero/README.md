# Kept revised publication: zero-block proof plus exact nonzero checks

Measured dcd16d266892c34cc7bcb03c1b2f8b7425360c74;
[green focused run40326741150](https://github.com/amanosatosi/taiyou/actions/runs/40326741150),
job121585288709, artifact12651643630. Paired parentbc7a38b57c29b7fef54a8d0b40e6cfe8bacd4580.
All2604 primary frames complete; no RGBA/diagnostic or original Taiyou digest
changes; no cache OFF/ON normal-limit result changes. Native tests pass, including
retained leases, capacity rejection, all channel/alpha combinations and block/tail
boundaries. CI formatting applied after recovery. Full gates remain unsatisfied
for this authorized draft probe;101 historical frames still require full audit.

This replaces the rejected whole-frame branchless reduction. Every byte of a
64-byte all-zero block is checked; zero RGBA is necessarily premultiplied. Every
nonzero block/tail uses the original channel validator. Shared boxed ownership
avoids the full pixel allocation copy, without retaining uncharged Vec capacity.
All former conservative transient reservations remain in place.

Paired parent->candidate median ms: dialogue1080p5.573->3.750; dialogue4K19.159->10.020;
blur4K23.852->14.598; transforms4K changing65.583->50.296; stress4K changing398.151->379.975.
Borders1080p8.796->8.242, so the previous experimental border slowdown is absent.
Keep this layer; it does not resolve the much larger serial raster architecture gap.
All states/distributions/areas/image counts are adjacent. These older tables use
the prior libass+host-RGBA comparison policy; the later direct-core comparison is
retained separately in results/mangetsu-core. Do not silently relabel old numbers.
