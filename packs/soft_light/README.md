# Soft Light

A light shader pack for phones. The sun lights every face by its angle and the open air glows,
without a shadow map; one full-screen pass draws the End's sky and does nothing elsewhere.

- Bright sunlight and moonlight that light each face by its angle, and a cool blue shade.
- Soft extra shade under trees, roofs and overhangs by day.
- Golden sunlight at sunrise and sunset, cool moonlight at night.
- Warm torch light with a gentle flicker.
- A clear sky gradient, blended with vanilla's sky so it meets the fog.
- Water with waves, a mirrored sky and a sun glint.
- A soft tone mapping, saturation and contrast you can set or turn off.

The End has a black sky with white stars and a slowly turning purple galaxy, and soft purple light; the Nether is warmer, with glowing torch light.

Movement (grass and flowers bending, leaves rustling, water rising and falling) needs a Horizon
build that supports a pack's `sway` function; on any other build the pack still works and nothing
moves.

## Credits

Written for Horizon by Muhamed (Muhamed Al-zarqi) and Claude. The ideas follow the Newb-SL Shader
(Newb) by Devendran S S, VeteranDev and KaruJoss, which is MIT licensed: the sky colours and
gradient, the soft shade rule, the extended Reinhard tone curve and the Nether and End light colours come from it, rewritten in WGSL. The End's galaxy swirl is our own spiral (Newb's was taken from a Shadertoy snippet whose licence we cannot confirm, so it is not copied).
No code of Minecraft or of any other shader pack is included. See `LICENSE.txt`.
