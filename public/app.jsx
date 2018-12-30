class ErrorBoundary extends React.Component {
    constructor(props) {
        super(props);
        this.state = { error: null, errorInfo: null };
    }

    componentDidCatchAAAAA(error, info) {
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
    render() {
        if (this.props.status === undefined) {
            return <div className="card">[Waiting for data]</div>;
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
                    <h5 className="card-title">{this.props.status.song.title}</h5>
                    <p className="card-text">{this.props.status.song.artist}</p>
                    <button className={"btn " + (paused ? "btn-primary" : "btn-secondary")} onClick={this.resume}>&gt;</button>
                    <button className={"btn " + (!paused ? "btn-primary" : "btn-secondary")} onClick={this.pause}>||</button>
                    <button className="btn btn-danger">Vote to skip</button>
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
                <img src="https://i.scdn.co/image/dcce0a15014719c88fac01d00d3921a17037467b" className="mr-3" alt="Album art" width="32px" />
                <b>{this.props.song}</b> by <b>{this.props.artist}</b>
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
                        {Object.keys(this.props.queue.songs).map((s) => <UpcomingListItem key={s} song={s} />)}
                    </ul>
                </div>
            );
        } else {
            var body = (
                <div>
                    <h2>No songs?!</h2>
                    <button className="btn btn-outline-info" type="button" onClick={this.props.showSearch}>Add song</button>
                </div>
            )
        }
        return (
            <div className="card">
                <div className="card-body">
                    {body}
                </div>
            </div>
        );
    }
}

class SearchWidget extends React.Component {
    constructor(props) {
        super(props);
        this.state = { value: '', data: [] };

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

        var u = "/search/track/" + encodeURIComponent(this.state.value);
        fetch(u).then(function (resp) {
            return resp.json(); // FIXME: Handle error
        }).then(function (d) {
            self.setState({ data: d });
        })
    }

    play(event) {
        event.preventDefault();
        console.log(event.currentTarget);
        let spotify_uri = event.currentTarget.dataset.spotifyurl;
        var u = "/api/request/" + encodeURIComponent(spotify_uri);
        fetch(u).then(function (resp) {
            return resp.json(); // FIXME: Handle error
        }.bind(this)).then(function (d) {
            console.log(d);
        }.bind(this));
    }

    render() {
        // Format search results if any
        console.log(this.state.data.Search);
        if (this.state.data.Search && this.state.data.Search.items.length > 0) {
            var sr = <ul className="list-group">
                {this.state.data.Search.items.map(
                    (x) => <li className="list-group-item" key={x.spotify_uri}>
                        <a href="#" onClick={this.play.bind(this)} data-spotifyurl={x.spotify_uri}>
                            <img src={x.album_image_url} width={32} />
                            <b>{x.title}</b> by <b>{x.artist}</b> ({formatDuration(x.duration_ms/1000)})
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
                        <input type="submit" value="Submit" />
                    </form>
                    {sr}
                    <button className="btn btn-outline-secondary" type="button" onClick={this.cancel.bind(this)}>Cancel</button>
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

    render() {
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
                    <span>=)</span>
                </nav>
                <p></p>
                {body}
            </ErrorBoundary>
        );
    }
}

ReactDOM.render(
    <MainView />,
    document.getElementById("app")
);