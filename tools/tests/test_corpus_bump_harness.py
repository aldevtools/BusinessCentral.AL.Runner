#!/usr/bin/env python3
"""Test of tools/corpus_bump_harness.py: python3 tools/tests/test_corpus_bump_harness.py"""
import os, sys, unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import corpus_bump_harness as h

FIXTURE = '''    let r = json("29");
    assert_eq!(r["overall"]["total"], 25077);
    assert_eq!(r["areas"]["record"]["total"], 3905); // corpus bump to 833a01ce
    assert!(md.starts_with("- BC 29.0: ") && md.contains(" of 25077 ("), "{md}");
    // of 25077 (events/platform-triggers/PeVTests.al:750, the RDLC holds lifted) and 85312
    // Corpus bump 55: tests of 25077 (AhDatasetHeader.al:309-430), pin oldsha
    for (v, total) in [("27", 24793), ("28", 24934), ("29", 25077)] {
    let r = json("29");
    assert_eq!(r["overall"]["total"], 25077);
    let p = "oldsha";
'''


class T(unittest.TestCase):
    def test_only_assertions_change(self):
        legs = {"27": "1", "28": "2", "29": "25536"}
        out = h.update(FIXTURE, legs, {"28": "9"}, "oldsha", "newsha", {"json": {"total": 5}, "record": {"total": 7}})
        a, b = FIXTURE.splitlines(), out.splitlines()
        self.assertEqual(len(a), len(b))
        for x, y in zip(a, b):
            if x.lstrip().startswith("//"):
                self.assertEqual(x, y)
        self.assertIn('md.contains(" of 25536 (")', out)
        self.assertIn('[("27", 1), ("28", 2), ("29", 25536)]', out)
        self.assertIn('["record"]["total"], 7);', out)
        self.assertIn('"newsha"', out)
        self.assertEqual(out.count("of 25077 ("), 2)  # the two comments
        self.assertNotIn("25077", out.replace("of 25077 (", ""))


if __name__ == "__main__":
    unittest.main()
