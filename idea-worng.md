I said:



```

fren, i have a brief theory based side question:

\- mechanically there are limits to the speed we can wiggle our square wave at- before inherent slew of the elec signal overcomes the squareness we can chieve of a waveform at a given frequency and hence we end up with near as damnit a sine or flatter and it ceases to be of sufficient accuracy crossing trailing edge/leading edge to trigger accurately and be useful at all. right?

\- so. what i'm wondering is; can we run two offset \[in time domain] square waves of same amplitude against eachother, then i'm thinking we can subtract any constructive/destructive interference resulting in lower/higher signal amplitude and hence derive where there should be a leading/trailing edge that's getting lost due slew? yis? would this work, is it a feasible solution to increasing data transfer rates as far as raw speed we can run a square wave at goes?

\- the drawback of this is it may preclude burying multiple signals in a single datasteam or multiple streams of data being used to err check eachother in a std. way - so it may not be advantageous overall compared to normal feasible gains now lost.

what do you think?

```





ChatGPT said:

Thought for 25s

```

Yeah, your intuition is pointing in a good direction here, woflfren — but this specific trick won’t actually let you bypass the fundamental speed limit of the channel. It is very close to something people already do in high-speed links, though.



Let me go through your bullets one by one.



1\. “Square waves stop being square” – what actually matters



mechanically there are limits to the speed we can wiggle our square wave … slew makes it sinusoidal and edges get mushy, so we lose precise crossings



Pretty much, yes — with some nuance:



A “square wave” is just a sum of odd harmonics:

fundamental + 3rd + 5th + 7th + …



Real channels (traces, transformers, coupling caps, line impedance, etc.) are low-pass: they attenuate those higher harmonics.



At high enough frequency / long enough line, you mostly get the fundamental left → waveform looks almost sinusoidal.



On top of that, finite driver slew + RC of the line gives you additional edge rounding.



But: for digital, we don’t actually need a perfect square — we only need:



* Enough eye opening (voltage margin) at the sampling instant, and
* Enough timing margin (jitter small relative to UI).



So yes, there’s a practical upper bound where the combination of bandwidth + SNR + jitter makes symbols no longer distinguishable. That’s the “it stopped being useful” point you’re describing.



2\. Two time-offset square waves \& subtraction



can we run two offset square waves … subtract constructive/destructive interference to recover edges lost to slew?



Short answer:



* Mathematically: what you’re describing is taking a difference between a signal and a delayed copy → a discrete-time derivative / high-pass.
* Physically: this is basically equalization / edge-enhancement, not a way to beat the channel limit.



Think of this as:



Let the received signal be 

𝑟(𝑡) (already rounded by the channel).



Form 𝑦(𝑡)=𝑟(𝑡)−𝑟(𝑡−Δ𝑡).



That’s exactly what a simple FIR equalizer with taps 

\[1,−1] does. In z-domain:



𝐻(𝑧)=1−𝑧−1



Magnitude response:



∣𝐻(𝑒𝑗𝜔)∣=2∣sin(𝜔/2)∣ → high-pass filter.



So you’re boosting high-frequency content (the edges) relative to the low-frequency content (the flat parts).



This is very close to:



* Pre-emphasis / de-emphasis in PCIe, SATA, HDMI, etc.
* CTLE / FFE equalizers in high-speed serial links.
* XOR + delay edge detection in plain digital logic (edge = signal XOR delayed(signal) is the same idea in 1-bit form).



So: as a receiver trick, yes, difference of delayed copies can sharpen transitions that are attenuated but still present. It can help claw back eye opening that the raw channel would hide. That’s legit.



But crucial point:

if the channel simply does not pass those harmonics at all (or they are buried in noise), no amount of subtraction / post-processing brings them back.



You’re still bounded by:



* Channel bandwidth,
* SNR,
* Nyquist / Shannon.



You can get closer to that limit with clever encoding and equalization, but you don’t hop over it.



3\. Does it increase “max square-wave speed” / data rate?



is it a feasible solution to increasing data transfer rates as far as raw speed we can run a square wave at goes?



Not in the “cheat physics” sense.



What you can do with this kind of idea:

* Equalize: compensate some of the channel roll-off, so you can run a given data rate more reliably.
* Push closer to the limit: take a channel that was marginal and make it acceptable at a given bitrate.
* Implement partial-response / duobinary-like schemes, where ISI is controlled and then cancelled digitally.



But it does not:



* Expand the channel bandwidth,
* Increase SNR,
* Or allow you to exceed the usual constraints like “symbol rate ≲ 2 × bandwidth”.



In other words: it’s a nice implementation technique (and already widely used in disguise), but not a new modality that lets you suddenly clock edges at 10× the old rate.



4\. Trade-offs / interaction with multiplexing



may preclude burying multiple signals in a single datastream / error-checking streams

Because the operation is linear (subtract a delayed copy), it plays fairly nicely with linear multiplexing. The real constraints are:

Extra noise amplification: high-pass behavior means you also boost HF noise; that tightens SNR requirements.

More sensitivity to timing error: delay mismatch or jitter in your “reference copy” shows up as extra distortion.

More complex detection when you have multi-level or multi-stream coding; you end up wanting a proper equalizer, not just this one trick.



In practice, high-speed links already:



* Use multi-level signaling (PAM-4, etc.) to increase bits per symbol.
* Have pre-emphasis and equalization to compensate the channel.
* Use coding (scrambling, 8b/10b, etc.) to keep transitions frequent, help clock recovery, and aid error detection.
* So your two-square-wave/difference idea is conceptually inside that same toolbox, not fundamentally incompatible with multi-stream/multi-level stuff.



5\. How I’d map your intuition to “real” DSP tools



Your idea is basically:



“Edges carry the interesting information. The flat bits are boring. Can I detect edges by comparing time-shifted versions of the signal and ‘cancel’ the flats?”



DSP translation:



Yes: use a derivative / high-pass / finite-difference filter or correlated detection around the expected edge time.



It’s edge enhancement / equalization, and it’s very much a thing.



But the upper bound on how fast you can run is still governed by the channel BW + SNR + coding. You’re just spending more DSP to live nearer that edge.



So: great intuition, just don’t expect it to open up a fundamentally new speed regime that beats Nyquist/Shannon. It’s a technique to rescue high-frequency content the channel only partially killed, not a way to resurrect content that never made it through.



optional:



Sketch a tiny Rust example where you take a bandlimited “distorted square wave” and show how y\[n] = x\[n] - x\[n-1] sharpens the transitions in a simulated channel — kind of a concrete visualization of your idea in DSP terms.

