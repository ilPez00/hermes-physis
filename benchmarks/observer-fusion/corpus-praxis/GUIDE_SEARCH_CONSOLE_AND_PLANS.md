# Operator guide — Search Console, and growing the plan corpus

> Two steps, both outside the repo. Everything code-side is shipped: `/sitemap.xml`
> serves 16 URLs, `robots.txt` lists both sitemaps, and each of those 16 URLs now
> has its own `<title>`, description, canonical and (where it is a product) JSON-LD.
>
> Nothing below can be done by an agent: step 1 needs your Google login, step 2
> needs your judgement about what is worth publishing.

---

## Step 1 — Verify the property and submit the sitemaps

Until this is done there is **no impression data at all**, so the 60-day question
in `docs/SEO_NICHE_EVALUATION.md` ("is this an indexation problem or a content
problem?") cannot be answered. `dig +short TXT praxisweb.xyz` returns nothing
today, so the property has never been verified.

### 1a. Add the property

1. Go to <https://search.google.com/search-console>.
2. **Add property** → choose **Domain** (not "URL prefix"). Domain properties
   cover `praxisweb.xyz`, `www.praxisweb.xyz` and both schemes in one place, and
   they are the only type that can read the full link report.
3. It asks for a DNS TXT record. Leave that tab open.

### 1b. Add the TXT record in Cloudflare

`praxisweb.xyz` resolves through Cloudflare, so this is one form:

1. <https://dash.cloudflare.com> → **praxisweb.xyz** → **DNS** → **Records**.
2. **Add record**: Type `TXT`, Name `@`, Content = the `google-site-verification=…`
   string Search Console gave you, TTL **Auto**. Save.
3. Wait ~1 minute, then confirm from this box:

   ```bash
   dig +short TXT praxisweb.xyz | grep google-site-verification
   ```

   If that prints nothing, the record has not propagated — do not press Verify
   yet. Cloudflare is usually under 60 seconds.

4. Back in Search Console, press **Verify**. It should succeed immediately.

### 1c. Submit both sitemaps

**Sitemaps** in the left nav → enter the path only, not the whole URL:

- `sitemap.xml`
- `plans/sitemap.xml`

Both are already listed in `robots.txt`, which is how Google usually finds them,
but submitting them is what gives you the per-sitemap "discovered / indexed"
counts you will read in step 3.

### 1d. Ask for indexing on the five pages that matter

**URL inspection** (the search box at the top) → paste the full URL → **Request
indexing**. Do this for:

- `https://praxisweb.xyz/me`
- `https://praxisweb.xyz/physis`
- `https://praxisweb.xyz/plans`
- `https://praxisweb.xyz/agliardi/`
- `https://praxisweb.xyz/physics/`

Requesting indexing is a hint, not a command, and there is a soft daily quota.
Five is fine; do not paste all sixteen.

### 1e. Bing, in one click

<https://www.bing.com/webmasters> → **Import from Google Search Console**. It
carries the property and the sitemaps over. Bing also feeds DuckDuckGo, which is
the engine the niche probes in `SEO_NICHE_EVALUATION.md` were run on.

### 1f. Verify from the outside, not from the dashboard

```bash
curl -s https://praxisweb.xyz/sitemap.xml | grep -c '<loc>'          # 16
curl -s https://praxisweb.xyz/robots.txt | grep Sitemap              # both sitemaps
curl -s https://praxisweb.xyz/me | grep -o '<title>[^<]*'            # Giovanni Pezzin — …
curl -s https://praxisweb.xyz/bioenergetics | grep -o '<title>[^<]*' # Bioenergetics — …
```

Every one of the 16 URLs answers 200 with no redirect (checked 2026-10-25), and
every SPA route among them now serves its own title rather than the app's.

---

## Step 2 — Grow the plan corpus

This is the step the earlier note got wrong. **There is nothing to publish.**

`/plans` renders rows from `goal_plan_suggestions`, and the table holds **36
rows**:

| rows | what they are                                                          |
| ---- | ---------------------------------------------------------------------- |
| 34   | e2e test debris — slugs like `e-2-e-1786527622339-run-a-half-marathon` |
| 1    | `run-a-half-marathon` — real, and already public                       |
| 1    | `bench-press-100-kg` — real, unpublished                               |

So "publish more plans" was a no-op: there is exactly one real unpublished plan.
The corpus has to be _grown_, and that is a content decision, not a flag.

### 2a. Clean the debris first

The 34 rows are test fixtures that leak into any future admin list. Read them
before deleting (destructive — that is why an agent did not run it):

```sql
select id, slug, goal_key, created_at
  from goal_plan_suggestions
 where slug like 'e-2-e-%'
 order by created_at
 limit 5;
```

Then, once you have seen that they are all `e-2-e-*` fixtures:

```sql
delete from goal_plan_suggestions where slug like 'e-2-e-%';
```

Optional but worth doing the same day — stop the e2e suite writing into the
production table (it is the reason 34 of 36 rows are junk):

```sql
-- check first: this is the constraint the e2e inserts depend on
select conname from pg_constraint
 where conrelid = 'goal_plan_suggestions'::regclass and contype = 'c';
```

### 2b. Decide what a plan is _for_

A `/plans/<slug>` page earns its place only if someone would search for that
exact goal. The page is server-rendered with `HowTo` schema, so it can rank; what
it cannot do is rank for a goal nobody types.

Ten goals worth checking against autocomplete before writing (the probe is
`https://suggestqueries.google.com/complete/search?client=firefox&q=…`, the same
one used for the niche table):

| candidate goal             | why                                                                                |
| -------------------------- | ---------------------------------------------------------------------------------- |
| `couch to 5k`              | enormous, evergreen, and the existing half-marathon plan already proves the format |
| `run a half marathon`      | already written — the reference for voice and step granularity                     |
| `bench press 100 kg`       | already written, unpublished                                                       |
| `dormire otto ore`         | the only Italian row that survived, and Italian search is less contested           |
| `learn to touch type`      | measurable, 4–8 weeks, no equipment                                                |
| `do 10 pull ups`           | same shape as the bench-press plan, higher search volume                           |
| `read 12 books in a year`  | non-physical, tests whether the format generalises                                 |
| `quit smoking`             | huge volume — and YMYL, so decide deliberately whether you want it                 |
| `learn 1000 Italian words` | fits the existing Italian audience                                                 |
| `save 5000 euro`           | money, so the same YMYL caution as above                                           |

Write the ones you would defend. Three good plans beat thirty generated ones, and
a generated plan is exactly the farm content `SEO_NICHE_EVALUATION.md` argues
against entering.

### 2c. Publish one

The route is `POST /api/admin/plans/:slug/publish` (`src/routes/adminRoutes.ts`)
and it takes a **JWT plus `requireAdmin`** — not an admin secret:

```bash
curl -X POST https://praxisweb.xyz/api/admin/plans/bench-press-100-kg/publish \
  -H "Authorization: Bearer $ADMIN_JWT" \
  -H 'Content-Type: application/json' \
  -d '{"publish":true}'
```

Or straight in SQL, which skips the review step the controller exists to enforce:

```sql
update goal_plan_suggestions
   set is_public = true, published_at = now()
 where slug = 'bench-press-100-kg';
```

Read the plan before publishing it. The controller treats publishing as a
deliberate act and so should you.

### 2d. Verify

```bash
curl -s https://praxisweb.xyz/plans/sitemap.xml | grep -c '<loc>'   # grows by 1
curl -s https://praxisweb.xyz/plans/bench-press-100-kg | head -20    # server-rendered HTML
```

The plan sitemap is generated from the database on each request, so a published
row appears without a deploy. Resubmit `plans/sitemap.xml` in Search Console
only if the count has not moved after a day.

---

## Step 3 — The 60-day scoreboard (do this on 2026-12-24)

Two numbers, both from Search Console, both already collected for you:

1. **Pages → Indexed.** If it is under 10 of the 16, the problem is indexation
   and the answer is links, not content. If it is 14–16, the sitemap worked and
   the question becomes whether any page _earns impressions_.
2. **Performance → Impressions over the last 28 days, and the queries.** Look for
   anything with impressions and no clicks: that is demand you are already close
   to.

Decision rule, written down now so it is not renegotiated later:

| what you see at 60 days                  | what it means                       | what to do                                                                                   |
| ---------------------------------------- | ----------------------------------- | -------------------------------------------------------------------------------------------- |
| < 10 pages indexed                       | indexation problem                  | get real inbound links (GitHub README, HN comment, a forum answer) — do not write more pages |
| indexed, < 100 impressions/month         | nobody is searching for these pages | stop; the domain is a shop window, not a funnel                                              |
| indexed, one query with real impressions | you found a niche by accident       | write the _one_ page that query wants, measure again                                         |
| a plan page outranking the rest          | the `/plans` format is the wedge    | grow that corpus, nothing else                                                               |

The honest expectation, from `SEO_NICHE_EVALUATION.md`: this is a credibility
surface for contract conversations, not income. Two days of contract work at the
€550 target is the same money as 10,000 qualified visits a month at 0.1 %
conversion — and 10,000 qualified visits is a 6–12 month build on a new domain.

---

## Decisions this guide cannot make for you

1. **Do you want YMYL topics** (`quit smoking`, `save 5000 euro`) on a domain
   whose front page is a goal tracker? They have the volume; they also invite a
   quality bar you would have to meet in public.
2. **Italian or English** for new plans. The audience is Italian, the search
   competition is lower in Italian, and every existing plan is in English.
3. **Does `/plans` stay free and account-free?** It is the only funnel on the
   domain that works without a signup today. If it ever needs an account, the
   whole sitemap argument changes.
