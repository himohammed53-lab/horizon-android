# Soft Light

A very light shader pack for phones. It changes colours only: no shadow map, and its one
full-screen pass (the End's sky) does nothing outside the End.

- Soft shade under trees, roofs and overhangs by day.
- Golden sunlight at sunrise and sunset, cool moonlight at night.
- Warm torch light with a gentle flicker.
- A clear sky gradient, blended with vanilla's sky so it meets the fog.
- Water with waves, a mirrored sky and a sun glint.
- A soft tone mapping, saturation and contrast you can set or turn off.

The End has a purple sky with a slowly turning galaxy and purple light everywhere; the Nether is warmer, with glowing torch light.

Not included: plants and leaves that sway, and waves that move the water's shape (a pack cannot move
vertices yet; the waves here change only how the surface is lit).

## Credits

Written for Horizon by Muhamed (Muhamed Al-zarqi) and Claude. The ideas follow the Newb-SL Shader
(Newb) by Devendran S S, VeteranDev and KaruJoss, which is MIT licensed: the sky colours and
gradient, the soft shade rule, the extended Reinhard tone curve and the Nether and End light colours come from it, rewritten in WGSL. The End's galaxy swirl is our own spiral (Newb's was taken from a Shadertoy snippet whose licence we cannot confirm, so it is not copied).
No code of Minecraft or of any other shader pack is included. See `LICENSE.txt`.
