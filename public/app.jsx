class ErrorBoundary extends React.Component {
    constructor(props) {
        super(props);
        this.state = { error: null, errorInfo: null };
    }

    componentDidCatch(error, info) {
        this.setState({
            error: error,
            errorInfo: errorInfo
        });
    }

    clear(event) {
        event.preventDefault();
        this.setState({ error: null, errorInfo: null });
    }

    render() {
        if (this.state.errorInfo) {
            // Error path
            return (
                <div>
                    <h2>Something went wrong.</h2>
                    <details style={{ whiteSpace: 'pre-wrap' }}>
                        {this.state.error && this.state.error.toString()}
                        <br />
                        {this.state.errorInfo.componentStack}
                    </details>
                    <a href="#" onClick={this.clear}>Cool</a>
                </div>
            );
        }
        return this.props.children;
    }
}

function formatDuration(time) {
    // Hours, minutes and seconds
    var hrs = ~~(time / 3600);
    var mins = ~~((time % 3600) / 60);
    var secs = ~~time % 60;

    // Output like "1:01" or "4:03:59" or "123:03:59"
    var ret = "";

    if (hrs > 0) {
        ret += "" + hrs + ":" + (mins < 10 ? "0" : "");
    }

    ret += "" + mins + ":" + (secs < 10 ? "0" : "");
    ret += "" + secs;
    return ret;
}

class PlaybackStatus extends React.Component {
    resume() {
        fetch("/api/resume");
    }
    pause() {
        fetch("/api/pause");
    }
    skip() {
        fetch("/api/skip");
    }
    render() {
        if (this.props.status === undefined || this.props.status.song === null || this.props.status.progress_ms === null) {
            return <div className="card">[Waiting for data]</div>;
        }

        if (this.props.status.state == 'NeedsSong') {
            return (
            <div className="card">
                <img src="/static/thejuke.png" className="card-img-top" width="286px" alt="Album artwork" style={{background: "black"}} />
                <div className="progress">
                    <div className="progress-bar progress-bar-striped" role="progressbar" style={{ width: "100%" }} aria-valuenow={100} aria-valuemin="0" aria-valuemax="100">
                    </div>
                </div>
                <div className="card-body">
                    <button className={"btn " + (paused ? "btn-primary" : "btn-secondary")} onClick={this.resume}>&gt;</button>
                    <button className={"btn " + (!paused ? "btn-primary" : "btn-secondary")} onClick={this.pause}>||</button>
                    <button className="btn btn-danger" onClick={this.skip}>Skip</button>
                </div>
            </div>
            );
        }

        let paused = this.props.status.state == "Paused";
        let progress = 100 * (this.props.status.progress_ms / this.props.status.song.duration_ms);

        let time_current = formatDuration(this.props.status.progress_ms / 1000);
        let time_duration = formatDuration(this.props.status.song.duration_ms / 1000);

        return (
            <div className="card">
                <img src={this.props.status.song.album_image_url} className="card-img-top" width="286px" alt="Album artwork" />
                <div className="progress">
                    <div className={"progress-bar" + (paused ? " progress-bar-striped" : "")} role="progressbar" style={{ width: progress + "%" }} aria-valuenow={progress} aria-valuemin="0" aria-valuemax="100">
                        <small style={{ color: "black" }} className="justify-content-end d-flex position-absolute w-100">{time_duration}</small>
                        <small style={{ color: "black" }} className="justify-content-start d-flex position-absolute w-100">{time_current}</small>
                    </div>
                </div>
                <div className="card-body">
                    <button className={"btn " + (paused ? "btn-primary" : "btn-secondary")} onClick={this.resume}>&gt;</button>
                    <button className={"btn " + (!paused ? "btn-primary" : "btn-secondary")} onClick={this.pause}>||</button>
                    <button className="btn btn-danger" onClick={this.skip}>Skip</button>
                    <h5 className="card-title">{this.props.status.song.title}</h5>
                    <p className="card-text">{this.props.status.song.artist}</p>
                    <p><small> ({this.props.status.state}) {time_current} / {time_duration}</small></p>
                </div>
            </div>
        );
    }
}

class UpcomingListItem extends React.Component {
    render() {
        return (
            <li className="list-group-item">
                <img src={this.props.song.album_image_url} className="mr-3" alt="Album art" width="32px" />
                <b>{this.props.song.title}</b> by <b>{this.props.song.artist}</b>
                <div className="float-right">
                    <button className="small primary">+1</button>
                    <button className="small secondary">Booo</button>
                </div>

            </li>
        );
    }
}

class UpcomingList extends React.Component {
    render() {
        if (this.props.queue === undefined) {
            return <span>Nothing yet..</span>;
        }
        if (Object.keys(this.props.queue.songs).length) {
            var body = (
                <div>
                    <h2>Upcoming songs, in no particular order:</h2>
                    <ul className="list-group">
                        {Object.keys(this.props.queue.songs).map((k) => <UpcomingListItem key={k} song={this.props.queue.songs[k]} />)}
                    </ul>
                </div>
            );
        } else {
            var body = (
                <div>
                    <h2>No songs?!</h2>
                </div>
            )
        }
        return (
            <div className="card">
                <div className="card-body">
                    {body}
                    <button className="btn btn-outline-info" type="button" onClick={this.props.showSearch}>Add song</button>
                </div>
            </div>
        );
    }
}

class SearchWidget extends React.Component {
    constructor(props) {
        super(props);
        this.state = { value: '', data: [], busy: false };

        this.handleChange = this.handleChange.bind(this);
        this.handleSubmit = this.handleSubmit.bind(this);
    }
    cancel() {
        this.props.cancel();
    }

    escFunction(event) {
        if (event.keyCode === 27) {
            this.cancel();
        }
    }
    componentDidMount() {
        document.addEventListener("keydown", this.escFunction.bind(this), false);
    }
    componentWillUnmount() {
        document.removeEventListener("keydown", this.escFunction.bind(this), false);
    }
    clearResults() {
        this.setState({ data: [] });
    }
    handleChange(event) {
        this.clearResults();
        this.setState({ value: event.target.value });
    }
    handleSubmit(event) {
        self = this;
        event.preventDefault();
        this.clearResults();
        this.setState({busy: true});

        var u = "/search/track/" + encodeURIComponent(this.state.value);
        fetch(u).then(function (resp) {
            this.setState({busy: false});
            return resp.json(); // FIXME: Handle error
        }.bind(this)).then(function (d) {
            self.setState({ data: d });
        }.bind(this));
    }

    play(event) {
        event.preventDefault();
        let spotify_uri = event.currentTarget.dataset.spotifyurl;
        var u = "/api/request/" + encodeURIComponent(spotify_uri);
        this.setState({busy: true});
        fetch(u).then(function (resp) {
            this.cancel();
            return resp.json(); // FIXME: Handle error
        }.bind(this)).then(function (d) {
            console.log("Requested song", d);
        }.bind(this));
    }

    render() {
        if(this.state.busy) {
            return (
                <div className="card">
                    <div className="card-body">
                        <div>Please wait...</div>
                    </div>
                </div>
            );
        }
        // Format search results if any
        if (this.state.data.Search && this.state.data.Search.items.length > 0) {
            var sr = <ul className="list-group">
                {this.state.data.Search.items.map(
                    (x) => <li className="list-group-item" key={x.spotify_uri}>
                        <a href="#" onClick={this.play.bind(this)} data-spotifyurl={x.spotify_uri}>
                            <img src={x.album_image_url} width={32} />
                            <b>{x.title}</b> by <b>{x.artist}</b> ({formatDuration(x.duration_ms / 1000)})
                        </a>
                    </li>)}
            </ul>
        } else {
            var sr = <div></div>;
        }
        // Main form + results
        return (
            <div className="card">
                <div className="card-body">
                    <form onSubmit={this.handleSubmit}>
                        <label>
                            Name: <input type="text" value={this.state.value} onChange={this.handleChange} />
                        </label>
                        {this.state.value.length > 0 ? <input type="submit" value="Search!" className="btn btn-success" /> : <span />}
                    </form>
                    {sr}
                    <button className="btn btn-outline-secondary" type="button" onClick={this.cancel.bind(this)}>Cancel</button>
                </div>
            </div>
        );
    }
}


class SelectDevice extends React.Component {
    constructor(props) {
        super(props);
        this.state = { data: null };
    }
    componentDidMount() {
        this.timer = setInterval(this.refresh.bind(this), 1000);
        this.refresh();
    }
    componentWillUnmount() {
        clearInterval(this.timer);
    }
    refresh() {
        self = this;
        var u = "/api/device/list";
        fetch(u).then(function (resp) {
            return resp.json(); // FIXME: Handle error
        }).then(function (d) {
            self.setState({ data: d });
        })
    }

    setActive(event) {
        event.preventDefault();
        let id = event.currentTarget.dataset.id;
        var u = "/api/device/set/" + encodeURIComponent(id);
        fetch(u).then(function (resp) {
            return resp.json(); // FIXME: Handle error
        }.bind(this)).then(function (d) {
            console.log(d);
        }.bind(this));
    }

    render() {
        // Format search results if any
        if (this.state.data && this.state.data.DeviceList && this.state.data.DeviceList.items.length > 0) {
            var sr = <ul className="list-group">
                {this.state.data.DeviceList.items.map(
                    (x) => <li className="list-group-item" key={x.id}>
                        <a href="#" onClick={this.setActive.bind(this)} data-id={x.id}>
                            Play on <b>{x.name}</b>
                        </a>
                    </li>)}
            </ul>
        } else {
            var sr = <div>No active devices - ensure a desktop Spotify client is running and online</div>;
        }
        return (
            <div className="card">
                <div className="card-body">
                    <h2>Select playback device</h2>
                    {sr}
                </div>
            </div>
        );
    }
}

const CON_CONNECTED = 'connected';
const CON_DISCONNECTED = 'disconnected';
const CON_UNKNOWN = 'unknown';

const STATUS_UPDATE_INTERVAL_MS = 1000;

class MainView extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            connected: CON_UNKNOWN,
            info: undefined,
            socket: undefined,
            status: undefined,
            queue: undefined,
            is_searching: false,
        };
    }
    componentDidMount() {
        // Create connection for live stuff
        const sock_url = ((window.location.protocol === "https:") ? "wss://" : "ws://") + window.location.host + "/ws";
        const sock = new WebSocket(sock_url, "juke");
        this.setState({ socket: sock });

        sock.addEventListener('open', function (event) {
            // Set state, and request initial update
            this.setState({ connected: CON_CONNECTED });
            this.refresh();
        }.bind(this));
        sock.addEventListener('message', function (event) {
            this.update(JSON.parse(event.data));
        }.bind(this));
        sock.addEventListener('close', function (event) {
            console.log("Web socket disconnected");
            this.disconnected();
        }.bind(this));
        sock.addEventListener('error', function (event) {
            console.log("Web socket error");
            this.disconnected();
        }.bind(this));

        // Routine update
        this.timer = setInterval(this.refresh.bind(this), STATUS_UPDATE_INTERVAL_MS);
    }
    componentWillUnmount() {
        clearInterval(this.timer);
    }
    refresh() {
        this.state.socket.send('status');
        this.state.socket.send('queue'); // FIXME: Do this less often
    }
    disconnected() {
        console.log("Clearing connection!");
        this.setState({ connected: CON_DISCONNECTED });
        this.setState({ "info": undefined });
    }
    update(data) {
        if ("Status" in data) {
            this.setState({ status: data.Status });
        } else if ("Queue" in data) {
            this.setState({ queue: data.Queue });
        } else {
            console.warn("Unhandled data from socket", data)
        }
    }
    toggleSearch() {
        this.setState({ is_searching: !this.state.is_searching });
    }
    clearDevice() {
        if(confirm("Select new device?")) {
            fetch("/api/device/clear");
            this.setState({conected: CON_UNKNOWN});
        }
    }
    logout() {
        if(confirm("Are you SURE? Are you SURE?")) {
            fetch("/auth/destroy");
            this.setState({conected: CON_UNKNOWN});
        }
    }

    render() {
        if (this.state.connected == CON_DISCONNECTED) {
            return <div className="card"><div className="card-item">[Lost conneciton to server. Try <a href="/">reloading?</a>]</div></div>;
        }
        if (this.state.status === undefined) {
            return <div className="card"><div className="card-item">[Waiting for data]</div></div>;
        }
        if (this.state.status.state == 'NoAuth') {
            return <div className="card"><div className="card-item">[Need authentication!] <a href="/auth">Host must log in with Spotify!</a></div></div>;
        }
        if (this.state.status.state == 'NoDevice') {
            return <SelectDevice />;
        }

        if (this.state.is_searching) {
            var body = (
                <div className="row">
                    <div className="col">
                        <SearchWidget cancel={this.toggleSearch.bind(this)} />
                    </div>
                </div>
            );
        } else {
            var body = (
                <div className="row">
                    <div className="col-md-4">
                        <PlaybackStatus status={this.state.status} />
                    </div>
                    <div className="col-md-8">
                        <UpcomingList queue={this.state.queue} showSearch={this.toggleSearch.bind(this)} />
                    </div>
                </div>
            );
        }
        return (
            <ErrorBoundary>
                <nav className="navbar navbar-dark bg-dark">
                    <a className="navbar-brand" href="#">Count Jukeula</a>
                    <button className={"btn btn-outline-info" + (this.state.is_searching ? ' active' : '')} type="button" onClick={this.toggleSearch.bind(this)}>Add song</button>
                    <span><img src="/static/thejuke.png" width="32px" /></span>
                </nav>
                <p></p>
                {body}
                <p></p>
                <nav className="navbar navbar-dark bg-dark">
                    <small>Count Jukeula the Chune Maker. Powered by Spotify. Vampire by Nikita Kozin from the Noun Project</small>
                    <small><a href="#" onClick={this.clearDevice.bind(this)}>Change device</a></small>
                    <small><a href="#" onClick={this.logout.bind(this)}>Disconnect from Spotify</a></small>
                </nav>
            </ErrorBoundary>
        );
    }
}

ReactDOM.render(
    <MainView />,
    document.getElementById("app")
);
