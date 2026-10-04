"""Adaptive soundtrack layers + natural sounds, synthesised (no licences needed).

Music layers (looped, mixed live by the game):
  m_drone   tanpura-style drone (Sa–Pa with jawari buzz)  — the climb
  m_bowls   sparse singing bowls                          — rests, camps, checkpoints
  m_tension frame-drum heartbeat + dissonant low cluster   — when danger is announced
  m_night   thin, cold high shimmer                         — night
  m_lament  placeholder vowel lament (replaced by real voices when samples arrive)
Natural sounds:
  step_snow*, step_rock*, step_ice*, flags (prayer-flag flutter loop), creak (serac ice loop)
"""
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, lfilter
import subprocess, os
SR = 32000
A = 'assets/audio/'
rng = np.random.default_rng(21)
TAU = 2 * np.pi


def t(d):
    return np.arange(int(SR * d)) / SR


def lp(x, f, o=2):
    b, a = butter(o, f / (SR / 2))
    return lfilter(b, a, x)


def hp(x, f, o=2):
    b, a = butter(o, f / (SR / 2), btype='high')
    return lfilter(b, a, x)


def bp(x, lo, hi, o=2):
    b, a = butter(o, [lo / (SR / 2), hi / (SR / 2)], btype='band')
    return lfilter(b, a, x)


def reverb(x, decay=2.5, mix=0.35):
    """Cheap mountain space: a few long feedback combs + allpass-ish smear."""
    out = np.zeros(len(x) + int(SR * decay))
    out[:len(x)] += x
    wet = np.zeros_like(out)
    for d, g in [(0.0297, 0.80), (0.0371, 0.78), (0.0411, 0.76), (0.0437, 0.74), (0.113, 0.6), (0.171, 0.5)]:
        n = int(d * SR)
        y = np.zeros_like(out)
        for i in range(0, len(out), n):
            seg = out[i:i + n]
            prev = y[i - n:i - n + len(seg)] if i >= n else 0
            y[i:i + len(seg)] = seg + g * prev
        wet += y
    wet = lp(wet / 6.0, 4500)
    return (1 - mix) * out + mix * wet


def loopify(x, total, fade=1.5):
    """Make a seamless loop of length `total` seconds: fold the tail back over the head."""
    n = int(total * SR)
    f = int(fade * SR)
    y = x[:n].copy()
    tail = x[n:n + f]
    if len(tail) < f:
        tail = np.pad(tail, (0, f - len(tail)))
    ramp = np.linspace(0, 1, f)
    y[:f] = y[:f] * ramp + tail * (1 - ramp) + y[:f] * 0  # tail fades out into the head
    y[:f] = x[:f] * ramp + tail * (1 - ramp)
    return y


def save(name, x, peak=0.9, stereo_width=0.0):
    x = x / (np.abs(x).max() + 1e-9) * peak
    if stereo_width > 0:
        d = int(0.011 * SR)
        r = np.concatenate([np.zeros(d), x[:-d]])
        st = np.stack([x, x * (1 - stereo_width) + r * stereo_width], 1)
    else:
        st = np.stack([x, x], 1)
    wav = f'/tmp/{name}.wav'
    wavfile.write(wav, SR, (st * 32767).astype(np.int16))
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', wav, '-c:a', 'libvorbis', '-q:a', '6', A + name + '.ogg'], check=True)
    os.remove(wav)


def note(m):  # midi -> Hz
    return 440 * 2 ** ((m - 69) / 12)


# ------------------------------------------------------------------ tanpura drone (D)
def pluck(f, dur, bright=1.0):
    tt = t(dur)
    y = np.zeros_like(tt)
    for n in range(1, 28):
        fn = f * n * (1 + 0.0004 * n * n)  # slight stiffness
        # jawari: upper partials swell after the attack, then die — the tanpura shimmer
        swell = (1 - np.exp(-tt * (1.5 + n * 0.25))) * np.exp(-tt * (0.35 + n * 0.05))
        a = (1 / n ** 0.6) * (0.6 + 0.4 * np.sin(n * 1.7) ** 2) * bright
        y += a * swell * np.sin(TAU * fn * tt + rng.uniform(0, TAU))
    tail = np.clip((dur - tt) / 1.5, 0, 1)  # soft end, no click
    return y * np.exp(-tt * 0.25) * tail


def drone():
    total = 24.0
    out = np.zeros(int(SR * (total + 6)))
    sa, pa, lsa = note(50), note(57), note(38)  # D3, A3, D2
    cycle = [pa, sa, sa, lsa]
    step = 1.5
    k = 0
    tt = 0.0
    while tt < total + 3:
        f = cycle[k % 4]
        p = pluck(f, 7.0, 0.9 if f != lsa else 1.1)
        i = int(tt * SR)
        out[i:i + len(p)] += p[:len(out) - i]
        tt += step
        k += 1
    # sustained bed underneath
    tb = np.arange(len(out)) / SR
    bed = 0.25 * np.sin(TAU * lsa * tb) + 0.12 * np.sin(TAU * sa * tb * 1.001) + 0.06 * np.sin(TAU * pa * tb * 0.999)
    out = lp(out, 3200) + bed * (0.8 + 0.2 * np.sin(TAU * tb / 9))
    out = reverb(out, 3.0, 0.4)
    save('m_drone', loopify(out, total, 3.0), 0.8, 0.5)


# ------------------------------------------------------------------ singing bowls
def bowl(f, dur=12.0):
    tt = t(dur)
    y = np.zeros_like(tt)
    for r, a, dcy in [(1, 1.0, 0.25), (2.71, 0.55, 0.4), (5.15, 0.25, 0.7), (8.1, 0.1, 1.1)]:
        for beat in (-0.6, 0.6):  # pairs of close modes: the slow wah of a real bowl
            y += a * np.exp(-tt * dcy) * np.sin(TAU * (f * r + beat * r) * tt + rng.uniform(0, TAU))
    strike = bp(rng.normal(0, 1, len(tt)), 1500, 6000) * np.exp(-tt * 60) * 0.3
    return y + strike


def bowls():
    total = 36.0
    out = np.zeros(int(SR * (total + 12)))
    for tt, f, g in [(0.5, note(62), 1.0), (12.5, note(57), 0.8), (24.0, note(65), 0.7)]:
        b = bowl(f) * g
        i = int(tt * SR)
        out[i:i + len(b)] += b[:len(out) - i]
    out = reverb(out, 3.5, 0.45)
    save('m_bowls', loopify(out, total, 4.0), 0.7, 0.6)


# ------------------------------------------------------------------ tension
def tension():
    total = 8.0
    tt = t(total + 2)
    out = np.zeros_like(tt)
    beat = 60 / 66
    for k in range(int((total + 2) / beat)):
        for off, g in ((0.0, 1.0), (0.24, 0.6)):  # lub-dub on a frame drum
            s0 = k * beat + off
            i = int(s0 * SR)
            d = t(0.6)
            hit = np.sin(TAU * (70 - 25 * d) * d) * np.exp(-d * 9) + 0.25 * bp(rng.normal(0, 1, len(d)), 200, 1200) * np.exp(-d * 30)
            out[i:i + len(d)] += g * hit[:len(out) - i]
    cluster = sum(np.sin(TAU * note(m) * tt) for m in (38, 39, 45)) * 0.12 * (0.7 + 0.3 * np.sin(TAU * tt / 4))
    out = lp(out + cluster, 1800)
    out = reverb(out, 1.5, 0.25)
    save('m_tension', loopify(out, total, 1.0), 0.8, 0.3)


# ------------------------------------------------------------------ night shimmer
def night():
    total = 30.0
    tt = t(total + 4)
    y = np.zeros_like(tt)
    for m, ph, rate in [(81, 0, 7.1), (88, 1.3, 9.3), (76, 2.1, 11.7), (93, 0.7, 5.3)]:
        y += np.sin(TAU * note(m) * tt + ph) * (0.5 + 0.5 * np.sin(TAU * tt / rate + ph)) ** 3 * 0.25
    air = bp(rng.normal(0, 1, len(tt)), 3000, 9000) * 0.05 * (0.6 + 0.4 * np.sin(TAU * tt / 13))
    out = reverb(y + air, 3.0, 0.5)
    save('m_night', loopify(out, total, 3.0), 0.5, 0.8)


# ------------------------------------------------------------------ lament (placeholder voice)
def vowel(f0, dur, formants=((320, 1.0, 80), (800, 0.45, 100), (2600, 0.08, 160))):
    tt = t(dur)
    vib = 1 + 0.006 * np.sin(TAU * 5.2 * tt) * np.clip(tt / 0.6, 0, 1)
    ph = np.cumsum(TAU * f0 * vib / SR)
    y = np.zeros_like(tt)
    for n in range(1, 30):
        fn = f0 * n
        a = sum(g / (1 + ((fn - F) / bw) ** 2) for F, g, bw in formants)
        y += a * np.sin(n * ph)
    env = np.clip(tt / 0.5, 0, 1) * np.clip((dur - tt) / 0.9, 0, 1)
    breath = bp(rng.normal(0, 1, len(tt)), 600, 3500) * 0.03
    return (y + breath) * env


def lament():
    # bhairavi-flavoured phrase on D: D Eb F G A Bb C — "oh... lo..."
    total = 28.0
    out = np.zeros(int(SR * (total + 6)))
    phrase = [(0.5, 74, 2.6), (3.3, 72, 1.4), (4.8, 70, 1.6), (6.6, 69, 3.4), (11.5, 77, 2.2), (13.8, 75, 1.2), (15.1, 74, 3.8), (20.5, 69, 1.6), (22.2, 70, 1.2), (23.5, 62, 4.0)]
    for st, m, d in phrase:
        v = vowel(note(m - 12), d)
        i = int(st * SR)
        out[i:i + len(v)] += v[:len(out) - i]
    out = reverb(lp(out, 5000), 4.0, 0.55)
    save('m_lament', loopify(out, total, 4.0), 0.6, 0.7)


# ------------------------------------------------------------------ footsteps & ambiences
def steps():
    for k in range(3):
        d = t(0.18)
        crunch = bp(rng.normal(0, 1, len(d)), 700, 6000) * (rng.random(len(d)) > 0.45) * np.exp(-d * 22)
        squeak = 0.15 * np.sin(TAU * rng.uniform(900, 1300) * d) * np.exp(-d * 40)
        save(f'step_snow{k}', crunch + squeak, 0.5)
        grit = hp(rng.normal(0, 1, len(d)), 2500) * (rng.random(len(d)) > 0.8) * np.exp(-d * 30)
        thud = np.sin(TAU * 140 * d) * np.exp(-d * 45) * 0.6
        save(f'step_rock{k}', grit + thud, 0.45)
        scrape = bp(rng.normal(0, 1, len(d)), 3000, 9000) * np.exp(-d * 18)
        ting = 0.25 * np.sin(TAU * rng.uniform(3200, 4200) * d) * np.exp(-d * 35)
        save(f'step_ice{k}', scrape + ting, 0.4)


def flags():
    total = 10.0
    tt = t(total + 1)
    flap = (0.5 + 0.5 * np.sin(TAU * 6.5 * tt + 2 * np.sin(TAU * 0.7 * tt))) ** 4
    gusts = 0.5 + 0.5 * np.sin(TAU * tt / 3.3) * np.sin(TAU * tt / 1.7)
    y = bp(rng.normal(0, 1, len(tt)), 400, 3000) * flap * gusts
    save('flags', loopify(y, total, 1.0), 0.6, 0.5)


def creak():
    total = 12.0
    tt = t(total + 2)
    y = np.zeros_like(tt)
    for st in rng.uniform(0, total, 6):
        d = t(rng.uniform(0.4, 1.2))
        f = rng.uniform(70, 160)
        c = np.sin(TAU * f * d * (1 + 0.3 * d)) * (rng.random(len(d)) > 0.6) * np.sin(np.pi * d / d[-1])
        i = int(st * SR)
        y[i:i + len(d)] += c[:len(y) - i]
    y = reverb(lp(y, 900), 2.0, 0.4)
    save('creak', loopify(y, total, 1.0), 0.6, 0.4)


if __name__ == '__main__':
    drone(); bowls(); tension(); night(); lament(); steps(); flags(); creak()
    print('music ok')


# ------------------------------------------------------------------ lament from real voices
def load(path):
    """Decode any audio file to mono float at SR via ffmpeg."""
    raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', path, '-ac', '1', '-ar', str(SR), '-f', 's16le', '-'], capture_output=True, check=True).stdout
    x = np.frombuffer(raw, np.int16).astype(float) / 32768
    return x / (np.abs(x).max() + 1e-9)


def repitch(x, semis):
    """Pitch by resampling (also changes length — slower and lower, like a voice far away)."""
    from scipy.signal import resample_poly
    from fractions import Fraction
    r = Fraction(2 ** (-semis / 12)).limit_denominator(64)
    return resample_poly(x, r.numerator, r.denominator)


def place(out, x, at, gain):
    i = int(at * SR)
    n = min(len(x), len(out) - i)
    if n > 0:
        out[i:i + n] += x[:n] * gain


def lament_voices():
    """Himalayan bed + Nordic call: Tibetan overtone chant underneath, a slowed female 'ooh'
    as a chord, and one kulning call far away, echoing off the mountain."""
    S = 'tools/samples/'
    chant, ooh, call = load(S + 'tibetan_monks_cc0.ogg'), load(S + 'female_ooh_ccby.ogg'), load(S + 'kulning_cc0.ogg')
    total = 40.0
    out = np.zeros(int(SR * (total + 10)))
    # bed: the chant, dark and low, the whole loop long
    bed = lp(chant[int(10 * SR):int(10 * SR) + len(out)], 1400)
    fade = np.clip(np.arange(len(bed)) / (SR * 2), 0, 1)
    place(out, bed * fade, 0.0, 0.45)
    # the 'ooh' slowed into a minor chord that swells and recedes
    for at, semis, g in [(1.0, -5, 0.55), (3.0, -12, 0.45), (5.5, -8, 0.35), (21.0, -7, 0.5), (23.5, -12, 0.4), (26.0, -10, 0.3)]:
        v = repitch(ooh, semis)
        env = np.sin(np.pi * np.clip(np.arange(len(v)) / len(v), 0, 1)) ** 1.5
        place(out, v * env, at, g)
    # the kulning: once, far away, then its echo off the far wall
    c = hp(call, 300)
    c = c / (np.abs(c).max() + 1e-9)
    place(out, lp(c, 5000), 12.0, 0.42)
    place(out, lp(c, 2500), 12.0 + 0.9, 0.16)
    out = reverb(out, 4.5, 0.55)
    save('m_lament', loopify(out, total, 5.0), 0.75, 0.7)


if __name__ == '__main__' and os.path.exists('tools/samples/kulning_cc0.ogg'):
    lament_voices()
    print('lament from voices ok')
