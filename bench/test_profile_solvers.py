import unittest

import profile_solvers as profile


class ProfileDriverTests(unittest.TestCase):
    def test_weight_range_changes_costs_but_not_topology(self):
        first = profile.graph_for("general", 40, 0.2, [1, 100], 17)
        second = profile.graph_for("general", 40, 0.2, [-1000, 1000], 17)
        self.assertEqual([(e["u"], e["v"]) for e in first["edges"]],
                         [(e["u"], e["v"]) for e in second["edges"]])
        self.assertNotEqual([e["weight"] for e in first["edges"]],
                            [e["weight"] for e in second["edges"]])
        planted = {(u, u + 1) for u in range(0, 40, 2)}
        self.assertTrue(planted.issubset({(e["u"], e["v"]) for e in first["edges"]}))

    def test_bipartite_generator_only_crosses_partitions(self):
        graph = profile.graph_for("bipartite", 20, 0.1, [1, 100], 23)
        self.assertTrue(all(e["u"] < 10 <= e["v"] for e in graph["edges"]))
        self.assertTrue({(u, 10 + u) for u in range(10)}.issubset(
            {(e["u"], e["v"]) for e in graph["edges"]}))

    def test_comparison_marks_every_disagreeing_group_member(self):
        rows = [
            {"solver": "primal-dual", "objective": "weight", "status": "ok",
             "total_weight": 2.0, "matching_size": 2, "is_perfect": True, "error": ""},
            {"solver": "blossom-engine", "objective": "weight", "status": "ok",
             "total_weight": 3.0, "matching_size": 2, "is_perfect": True, "error": ""},
        ]
        profile.compare_group(rows)
        self.assertEqual([r["status"] for r in rows], ["disagreement", "disagreement"])


if __name__ == "__main__":
    unittest.main()
