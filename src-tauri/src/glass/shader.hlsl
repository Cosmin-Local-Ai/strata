// Strata glass shader. Renders the dock's glass body from a copy of the desktop layer behind it.
// All coordinates are physical pixels in the dock window's client space unless noted.

Texture2D bg : register(t0);
SamplerState samp : register(s0);

cbuffer Params : register(b0)
{
    float4 rect;        // x, y, w, h of the glass body in window px
    float4 tint;        // rgb tint (straight), a = tint amount
    float2 winSize;     // window client size in px
    float  radius;      // corner radius px
    float  refraction;  // 0..1 strength of edge bending
    float2 bgOrigin;    // window origin inside the bg texture (px)
    float2 bgSize;      // bg texture size (px)
    float  blurLod;     // mip level used for frost (0 = clear)
    float  rim;         // specular rim intensity
    float  dispersion;  // chromatic split at the rim (px)
    float  bodyAlpha;   // 1 = glass visible, used for show/hide fades
    float4 shade;       // rgb + a: flat colour body used by solid styles (Black/White/Midnight), a = mix
};

struct VSOut { float4 pos : SV_Position; float2 uv : TEXCOORD0; };

VSOut vs_main(uint id : SV_VertexID)
{
    VSOut o;
    float2 uv = float2((id << 1) & 2, id & 2);
    o.pos = float4(uv * float2(2, -2) + float2(-1, 1), 0, 1);
    o.uv = uv;
    return o;
}

float sdRoundRect(float2 p, float2 b, float r)
{
    float2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

float3 sampleBg(float2 px, float lod)
{
    float2 uv = (px + bgOrigin) / bgSize;
    return bg.SampleLevel(samp, uv, lod).rgb;
}

float4 ps_main(VSOut i) : SV_Target
{
    float2 px = i.uv * winSize;
    float2 hb = rect.zw * 0.5;
    float2 c = rect.xy + hb;
    float r = min(radius, min(hb.x, hb.y));
    float2 p = px - c;
    float d = sdRoundRect(p, hb, r);
    float cover = saturate(0.5 - d);
    if (cover <= 0.0 || bodyAlpha <= 0.0) return float4(0, 0, 0, 0);

    // Surface normal from the SDF gradient (points outward).
    float e = 0.75;
    float2 g = float2(sdRoundRect(p + float2(e, 0), hb, r) - sdRoundRect(p - float2(e, 0), hb, r),
                      sdRoundRect(p + float2(0, e), hb, r) - sdRoundRect(p - float2(0, e), hb, r));
    float2 n = g / max(length(g), 1e-4);

    // Bevel band: the outer part of the body bends light like a thick convex lens edge.
    float band = max(min(r, min(hb.x, hb.y)) * 0.9, 4.0);
    float t = saturate(1.0 + d / band);          // 0 deep inside, 1 at the rim
    float bend = t * t * t * refraction * band;  // displacement grows sharply toward the rim
    float2 sp = px - n * bend;                   // sample from further inside: content is pulled toward the rim

    // Slight magnification of the interior, like a lens with a little thickness.
    sp = c + (sp - c) * (1.0 - 0.04 * refraction);

    float3 col;
    if (dispersion > 0.0)
    {
        float2 dsp = n * dispersion * t * t;
        col.r = sampleBg(sp + dsp, blurLod).r;
        col.g = sampleBg(sp, blurLod).g;
        col.b = sampleBg(sp - dsp, blurLod).b;
    }
    else
    {
        col = sampleBg(sp, blurLod);
    }

    col = lerp(col, tint.rgb, tint.a);
    col = lerp(col, shade.rgb, shade.a);

    // Specular rim: thin bright line, strongest where the normal faces the light (top-left).
    float rimLine = saturate(1.0 - abs(d + 0.9) / 1.2);
    float facing = 0.35 + 0.65 * saturate(dot(n, normalize(float2(-0.45, -1.0))));
    float inner = saturate(1.0 - abs(d + 2.5) / 2.5) * 0.35;
    col += rim * (rimLine * facing + inner * facing * 0.5);

    float a = cover * bodyAlpha;
    return float4(saturate(col) * a, a); // premultiplied
}
