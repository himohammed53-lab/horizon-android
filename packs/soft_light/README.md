# Soft Light

A very light shader pack for phones. It changes colours only: no shadow map and no extra
full-screen pass, so it costs very little.

- Soft shade under trees, roofs and overhangs by day.
- Golden sunlight at sunrise and sunset, cool moonlight at night.
- Warm torch light with a gentle flicker.
- A clear sky gradient, blended with vanilla's sky so it meets the fog.
- Water with waves, a mirrored sky and a sun glint.
- A soft tone mapping, saturation and contrast you can set or turn off.

Not included: plants and leaves that sway, and waves that move the water's shape (a pack cannot move
vertices yet; the waves here change only how the surface is lit).

## Credits

Written for Horizon by Muhamed (Muhamed Al-zarqi) and Claude. The ideas follow the Newb-SL Shader
(Newb) by Devendran S S, VeteranDev and KaruJoss, which is MIT licensed: the sky colours and
gradient, the soft shade rule and the extended Reinhard tone curve come from it, rewritten in WGSL.
No code of Minecraft or of any other shader pack is included. See `LICENSE.txt`.
