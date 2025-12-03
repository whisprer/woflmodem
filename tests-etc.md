In-process baseband loopback test:



Use your real modem DSP chain (scrambler → mapper → pulse shaping → channel → timing recovery → Costas → slicer → descrambler).



Drive it with a PRBS or known pattern.



Simulate a channel: AWGN + frequency offset + timing offset + maybe small filter to emulate POTS.



Assert that BER, symbol sync, and phase lock all look sane for Bell 103 / V.22 / V.22bis.



This gives you a clean “golden path” regression test that’s independent of Windows audio/TAPI weirdness.



Audio loopback on the same machine:



Once the baseband works:



Start a “modem TX” instance that plays the modulated audio out of your default output device.



Use Windows “Stereo Mix” or a virtual audio cable (VB-Audio, VoiceMeeter, etc.) to feed that into your input device.



Run your RX pipeline on the same box, see if you can decode the stream and match BER vs your offline sim.



This is the softest step into “real-time” without involving PSTN.









5\. How to benchmark speed / stability / accuracy



For Bell 103 / V.22 / V.22bis, you care about:



BER vs SNR curves for each profile.



Handshake success rate and time.



Timing / carrier lock robustness (how much jitter and offset you tolerate).



For HSF/FAX eventually: success rate on T.30 sessions, but that’s later.



A very practical test/benchmark plan:



5.1 Offline BER bench



Build a small “lab” binary or test module that:



Generates a long PRBS or random payload.



Feeds it through your modem TX chain at a given profile and SNR.



Passes the samples through your channel sim (AWGN + impairments).



Runs RX chain, recovers bits, compares to original.



Logs BER, lock time, and maybe “loss of lock” events.



You can sweep:



SNR from, say, 0 to 40 dB.



Frequency offset / timing offset.



Filter mis-match.



That gives you actual performance curves you can compare to what ITU specs \& app notes show for V.22bis modems under similar conditions. 

3amsystems.com

+1



5.2 Real-time stability bench



Once audio init is fixed and loopback works:



Run high-rate TX stream (e.g. V.22bis at 2400 bps) for many minutes.



Measure:



Packet/bit error rate over time.



Latency distribution in your DSP loop.



CPU usage on the real-time thread.



If you want quick sanity metrics:



Throughput: measure bits delivered to TAPI per second and compare to the theoretical (e.g. 2400 bps).



Errors: count CRC / HDLC frame errors or bad blocks per second.













