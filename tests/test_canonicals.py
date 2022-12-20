import unittest
import yaml
from pathlib import Path
from cooklang import parse

CANONICAL_TESTS_FILE = Path(__file__).parent / "canonical.yaml"


class TestCanonical(unittest.TestCase):
    def test_canonical(self):
        tests = yaml.safe_load(CANONICAL_TESTS_FILE.read_text())
        for name, test in tests["tests"].items():
            print(name)
            source = test["source"]
            result = test["result"]
            cooklang_result = parse(source)

            n_metadata_cooklang = 0
            line_index = 0
            for r in cooklang_result:
                if len(r) == 1 and "type" in r[0] and r[0]["type"] == "metadata":
                    # metadata
                    n_metadata_cooklang += 1
                    name = r[0]["key"]
                    self.assertTrue(name in result["metadata"])
                    self.assertEqual(result["metadata"][name], r[0]["value"] )
                else:
                    # parser don't output empty text, canonical does: remove empty text from canonical
                    canonical_steps = [
                        e_canonical
                        for e_canonical in result["steps"][line_index]
                        if not("type" in e_canonical 
                        and "value" in e_canonical 
                        and e_canonical["type"] == "text" 
                        and e_canonical["value"].strip() == "")
                    ]
                    

                    self.assertEqual(len(canonical_steps), len(r))
                    for e_canonical, e_parser in zip(canonical_steps, r):
                        if e_canonical["type"] == "text":
                            self.assertTrue("text" in e_parser)
                            self.assertEqual(e_parser["text"], e_canonical["value"].strip())
                        else:
                            # quantity is not managed the same way between canonical and parser
                            # - if quantity is not a string in canonical, transform it to string
                            # - if quantity is a default value, remove the default value
                            # - if quantity is a frac, then change it to string representation 
                            if 'quantity' in e_canonical:
                                e_canonical["quantity"] = str(e_canonical["quantity"])
                                if e_parser["quantity"] == "":
                                    self.assertIn(e_canonical["quantity"], ["1", "some"])
                                    e_canonical["quantity"] = ""
                                if e_parser["quantity"] != e_canonical["quantity"]:
                                    # then probably fraction
                                    self.assertEqual(eval(e_parser["quantity"]), eval(e_canonical["quantity"]))
                                    e_canonical["quantity"] = e_parser["quantity"]
                            self.assertEqual(e_canonical, e_parser)
                        
                    line_index += 1
            self.assertEqual(line_index, len(result["steps"]))
            self.assertEqual(n_metadata_cooklang, len(result["metadata"]))


