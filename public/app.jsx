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
    constructor(props) {
        super(props);
        this.state = { info: undefined };
    }
    componentDidMount() {
        const sock_url = ((window.location.protocol === "https:") ? "wss://" : "ws://") + window.location.host + "/ws";
        const sock = new WebSocket(sock_url, "juke");
        sock.addEventListener('open', function (event) {
            sock.send('status');
        });
        sock.addEventListener('message', function (event) {
            let d = event.data;
            this.update(JSON.parse(d));
        }.bind(this));
        sock.addEventListener('close', function (event) {
            this.disconnected();
        }.bind(this));
        // Trigger routine update
        this.timer = setInterval(function () { sock.send('status') }.bind(this), 1000);
    }
    componentWillUnmount() {
        clearInterval(this.timer);
    }
    disconnected() {
        this.setState({"info": undefined});
        throw new Error("Lost connection to server");
    }
    update(data) {
        console.log("Playback status got data", data);
        this.setState({ "info": data });
    }

    resume() {
        fetch("/api/resume");
    }
    pause() {
        fetch("/api/pause");
    }

    render() {
        if (this.state.info === undefined) {
            return <div className="card">[Connecting...!]</div>;
        }
        console.log(this.state.info);

        let paused = this.state.info.Status.state == "Paused";
        let progress = 100 * (this.state.info.Status.progress_ms / this.state.info.Status.song.duration_ms);

        let time_current = formatDuration(this.state.info.Status.progress_ms / 1000);
        let time_duration = formatDuration(this.state.info.Status.song.duration_ms / 1000);

        return (
            <div className="card">
                <img src={this.state.info.Status.song.album_image_url} className="card-img-top" width="286px" alt="Album artwork" />
                <div className="progress">
                    <div className={"progress-bar" + (paused ? " progress-bar-striped" : "")} role="progressbar" style={{ width: progress + "%" }} aria-valuenow={progress} aria-valuemin="0" aria-valuemax="100">
                        <small style={{ color: "black" }} className="justify-content-end d-flex position-absolute w-100">{time_duration}</small>
                        <small style={{ color: "black" }} className="justify-content-start d-flex position-absolute w-100">{time_current}</small>
                    </div>
                </div>
                <div className="card-body">
                    <h5 className="card-title">{this.state.info.Status.song.title}</h5>
                    <p className="card-text">{this.state.info.Status.song.artist}</p>
                    <button className={"btn " + (paused ? "btn-primary" : "btn-secondary")} onClick={this.resume}>&gt;</button>
                    <button className={"btn " + (!paused ? "btn-primary" : "btn-secondary")} onClick={this.pause}>||</button>
                    <button className="btn btn-danger">Vote to skip</button>
                    <p><small> ({this.state.info.Status.state}) {time_current} / {time_duration}</small></p>
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
        return (
            <div className="card">
                <div className="card-body">
                    <h2>Upcoming songs, in no particular order:</h2>
                    <ul className="list-group">
                        <UpcomingListItem song="1940" artist="The Submarines" />
                    </ul>
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

        var u = new URL("http://localhost:8081/search/track/" + encodeURIComponent(this.state.value));
        fetch(u).then(function (resp) {
            return resp.json(); // FIXME: Handle error
        }).then(function (d) {
            self.setState({ data: d });
        })
    }

    play(event) {
        event.preventDefault();
        let spotify_uri = event.target.value;
        console.log('k', spotify_uri)
        var u = new URL("http://localhost:8081/api/request/" + encodeURIComponent(spotify_uri));
        fetch(u).then(function (resp) {
            return resp.json(); // FIXME: Handle error
        }).then(function (d) {
            console.log(d);
        })
    }

    render() {
        // Format search results if any
        if (this.state.data.Search && this.state.data.Search.items.length > 0) {
            var sr = <ul>
                {this.state.data.Search.items.map(
                    (x) => <li key={x.spotify_uri}>
                        <a href="#" onClick={this.play} value={x.spotify_uri}>
                            <b>{x.name}</b> by <b>{x.artists}</b> ({x.spotify_uri}
                        </a>
                    </li>)}
            </ul>
        } else {
            var sr = <div></div>;
        }

        // Main form + results
        return (
            <div>
                <form onSubmit={this.handleSubmit}>
                    <label>
                        Name:
            <input type="text" value={this.state.value} onChange={this.handleChange} />
                    </label>
                    <input type="submit" value="Submit" />
                </form>
                {sr}
            </div>
        );
    }
}

class MainView extends React.Component {
    render() {
        return (
            <ErrorBoundary>
                <nav className="navbar navbar-dark bg-dark">
                    <a className="navbar-brand" href="#">Count Jukeula</a>
                    <button className="btn btn-outline-info" type="button">Add song</button>
                    <button className="btn btn-outline-secondary" type="button">Exit</button>
                </nav>
                <p></p>
                <div className="row">
                    <div className="col-md-4">
                        <PlaybackStatus />
                    </div>
                    <div className="col-md-8">
                        <UpcomingList />
                    </div>
                </div>
            </ErrorBoundary>
        );
    }
}

ReactDOM.render(
    <MainView />,
    document.getElementById("app")
);