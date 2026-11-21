# 📊 PRAXIS METRICS DASHBOARD — Admin Guide

## API Endpoint

**GET /api/admin/metrics?days=30**

Returns key metrics for acquisition packaging and growth tracking.

### Response Schema

```json
{
  "totalUsers": 1250,
  "activeUsers7d": 342,
  "activeUsers30d": 687,
  "payingUsers": 45,
  "mrr": 450,
  "checkinsThisPeriod": 2847,
  "totalGoals": 4521,
  "postsThisPeriod": 156,
  "achievementsThisPeriod": 89,
  "retentionCurve": [
    { "day": 0, "dau": 52 },
    { "day": 1, "dau": 48 },
    ...
  ],
  "topGoals": [
    { "name": "Fitness", "count": 892 },
    { "name": "Career", "count": 654 }
  ],
  "generatedAt": "2026-03-28T14:30:00.000Z"
}
```

### Key Metrics Definitions

| Metric           | Definition                          | Calculation                                                         |
| ---------------- | ----------------------------------- | ------------------------------------------------------------------- |
| **Total Users**  | All registered users                | COUNT(profiles)                                                     |
| **DAU (7d)**     | Unique users active in last 7 days  | COUNT(DISTINCT checkins.user_id) WHERE checked_in_at >= NOW() - 7d  |
| **MAU (30d)**    | Unique users active in last 30 days | COUNT(DISTINCT checkins.user_id) WHERE checked_in_at >= NOW() - 30d |
| **Paying Users** | Active Pro subscribers              | COUNT(user_subscriptions) WHERE status = 'active'                   |
| **MRR**          | Monthly Recurring Revenue           | payingUsers × $10 (avg subscription)                                |
| **Check-ins**    | Total daily check-ins               | COUNT(checkins) WHERE created_at >= since                           |
| **Total Goals**  | All goal nodes created              | COUNT(goal_trees.nodes)                                             |
| **Posts**        | Community posts created             | COUNT(posts) WHERE created_at >= since                              |
| **Achievements** | Unlocked achievements               | COUNT(user_achievements) WHERE completed = true                     |

---

## Frontend Integration (Admin Page)

Add to `client/src/features/admin/AdminPage.tsx`:

```tsx
// Add to StatsTab or create new MetricsTab
const MetricsDashboard = () => {
  const [metrics, setMetrics] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.get("/admin/metrics").then((r) => {
      setMetrics(r.data);
      setLoading(false);
    });
  }, []);

  if (loading) return <CircularProgress />;

  return (
    <Grid container spacing={3}>
      {/* Key Metrics Cards */}
      <Grid size={3}>
        <Card>
          <CardContent>
            <Typography variant="h3">{metrics.totalUsers}</Typography>
            <Typography variant="body2">Total Users</Typography>
          </CardContent>
        </Card>
      </Grid>
      <Grid size={3}>
        <Card>
          <CardContent>
            <Typography variant="h3">{metrics.mrr}</Typography>
            <Typography variant="body2">MRR ($)</Typography>
          </CardContent>
        </Card>
      </Grid>
      {/* ... more cards */}
    </Grid>
  );
};
```

---

## Export for Acquisition Packet

```javascript
// In AdminPage, add export button
const handleExportMetrics = async () => {
  const { data } = await api.get("/admin/metrics");
  const csv = [
    ["Metric", "Value"],
    ["Total Users", data.totalUsers],
    ["DAU (7d)", data.activeUsers7d],
    ["MAU (30d)", data.activeUsers30d],
    ["Paying Users", data.payingUsers],
    ["MRR", data.mrr],
    ["Check-ins (period)", data.checkinsThisPeriod],
  ]
    .map((row) => row.join(","))
    .join("\n");

  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `praxis-metrics-${new Date().toISOString().slice(0, 10)}.csv`;
  a.click();
};
```

---

## Usage for Acquisition

1. **Weekly Tracking**: Export metrics every Sunday
2. **Growth Trajectory**: Plot DAU/MAU over time
3. **Unit Economics**: Calculate LTV:CAC ratio
4. **Due Diligence**: Provide to potential acquirers

---

**Created:** March 28, 2026  
**Endpoint:** `GET /api/admin/metrics`  
**Auth:** Admin JWT required
