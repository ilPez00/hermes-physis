import json
from pathlib import Path

import pytest
import tools
from schemas import ALL_NAMES
from test_security import _learned_candidate, requires_replant


def test_ad_hoc_record_is_not_model_callable():
    assert 'replant_record' not in ALL_NAMES


def test_untrusted_store_cannot_trigger_model_replay():
    # The model can write files as the same OS user; mode 0700 is not an
    # authorization boundary. Only the operator slash command may replay.
    assert 'replant_run' not in ALL_NAMES


@requires_replant
def test_candidate_approval_rejects_tampered_check(tmp_path, monkeypatch):
    candidate = _learned_candidate(tmp_path, 'echo hi')
    marker = tmp_path / 'compromised'
    path = Path(tools._replant_store()) / 'candidates' / (candidate['id'] + '.json')
    data = json.loads(path.read_text())
    data['check'] = 'touch ' + str(marker)
    path.write_text(json.dumps(data))
    monkeypatch.chdir(tmp_path)
    result = json.loads(tools.make_physis_command({'timeout': 30})('approve ' + candidate['id']))
    assert result['ok'] is False
    assert not marker.exists()


@requires_replant
@pytest.mark.parametrize('damage', ['missing', 'corrupt', 'empty', 'invalid_digest'])
def test_replay_blocks_unverifiable_freshness(tmp_path, monkeypatch, damage):
    candidate = _learned_candidate(tmp_path, 'echo hi')
    monkeypatch.chdir(tmp_path)
    command = tools.make_physis_command({'timeout': 30})
    assert json.loads(command('approve ' + candidate['id']))['ok']
    sidecar = tools._artifacts_file({'input': 'greet the world'})
    if damage == 'missing':
        sidecar.unlink()
    elif damage == 'corrupt':
        sidecar.write_text('{')
    elif damage == 'empty':
        sidecar.write_text('{}')
    else:
        sidecar.write_text(json.dumps({'artifacts': {str(tmp_path / 'input.txt'): ''}}))
    result = json.loads(command('replay greet the world'))
    assert result.get('blocked') is True, result
    assert result.get('provider_calls') == 0


def test_oversize_artifact_is_not_recorded_as_valid(tmp_path):
    path = tmp_path / 'large'
    with path.open('wb') as f:
        f.truncate(tools.MAX_ARTIFACT_BYTES + 1)
    with pytest.raises((OSError, ValueError)):
        tools._write_artifacts({'input': 'x'}, [path])
